// Simulated annealing acceptor with lexicographic score-level temperatures.

use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{Score, ScoreLevel};

use super::Acceptor;
use crate::heuristic::r#move::MoveTabuSignature;

const DEFAULT_DECAY_RATE: f64 = 0.999985;
const DEFAULT_HILL_CLIMBING_TEMPERATURE: f64 = 1.0e-9;
const DEFAULT_CALIBRATION_SAMPLE_SIZE: usize = 128;
const DEFAULT_TARGET_ACCEPTANCE_PROBABILITY: f64 = 0.80;
const DEFAULT_FALLBACK_TEMPERATURE: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardRegressionPolicy {
    TemperatureControlled,
    NeverAcceptHardRegression,
}

#[derive(Debug, Clone, Copy)]
pub struct SimulatedAnnealingCalibration {
    pub sample_size: usize,
    pub target_acceptance_probability: f64,
    pub fallback_temperature: f64,
}

impl Default for SimulatedAnnealingCalibration {
    fn default() -> Self {
        Self {
            sample_size: DEFAULT_CALIBRATION_SAMPLE_SIZE,
            target_acceptance_probability: DEFAULT_TARGET_ACCEPTANCE_PROBABILITY,
            fallback_temperature: DEFAULT_FALLBACK_TEMPERATURE,
        }
    }
}

#[derive(Debug, Clone)]
enum TemperatureSeed {
    Single(f64),
    PerLevel(Vec<f64>),
    Calibrated(SimulatedAnnealingCalibration),
}

#[derive(Debug, Clone)]
struct CalibrationState {
    config: SimulatedAnnealingCalibration,
    samples_by_level: Vec<Vec<i64>>,
    samples_seen: usize,
}

impl CalibrationState {
    fn new(level_count: usize, config: SimulatedAnnealingCalibration) -> Self {
        let reserve_per_level = (config.sample_size / level_count.max(1)).max(1);
        Self {
            config,
            samples_by_level: (0..level_count)
                .map(|_| Vec::with_capacity(reserve_per_level))
                .collect(),
            samples_seen: 0,
        }
    }

    fn record(&mut self, level: usize, delta_abs: i64) -> bool {
        self.samples_by_level[level].push(delta_abs);
        self.samples_seen += 1;
        self.samples_seen >= self.config.sample_size
    }

    fn temperatures(&self) -> Vec<f64> {
        let denominator = -self.config.target_acceptance_probability.ln();
        self.samples_by_level
            .iter()
            .map(|samples| {
                if samples.is_empty() {
                    self.config.fallback_temperature
                } else {
                    let total: i128 = samples.iter().map(|&value| i128::from(value)).sum();
                    let mean = total as f64 / samples.len() as f64;
                    (mean / denominator).max(self.config.fallback_temperature)
                }
            })
            .collect()
    }
}

/* Simulated annealing acceptor using first-differing-level Boltzmann acceptance.

Improving or equal moves are accepted unconditionally. Worsening moves are
classified by the first score level that differs, and the acceptance probability
uses only that level's delta and temperature. Lower-priority levels never mask
higher-priority regressions.
*/
#[derive(Debug, Clone)]
pub struct SimulatedAnnealingAcceptor {
    temperature_seed: TemperatureSeed,
    starting_temperatures: Vec<f64>,
    current_temperatures: Vec<f64>,
    decay_rate: f64,
    hill_climbing_temperature: f64,
    hard_regression_policy: HardRegressionPolicy,
    calibration_state: Option<CalibrationState>,
    rng: SmallRng,
    level_count: usize,
}

impl SimulatedAnnealingAcceptor {
    /// Creates an acceptor with the same delta temperature for every score level.
    pub fn new(starting_temperature: f64, decay_rate: f64) -> Self {
        Self::from_seed(
            TemperatureSeed::Single(starting_temperature),
            decay_rate,
            DEFAULT_HILL_CLIMBING_TEMPERATURE,
            HardRegressionPolicy::TemperatureControlled,
            SmallRng::from_rng(&mut rand::rng()),
        )
    }

    pub fn with_seed(starting_temperature: f64, decay_rate: f64, seed: u64) -> Self {
        Self::from_seed(
            TemperatureSeed::Single(starting_temperature),
            decay_rate,
            DEFAULT_HILL_CLIMBING_TEMPERATURE,
            HardRegressionPolicy::TemperatureControlled,
            SmallRng::seed_from_u64(seed),
        )
    }

    pub fn with_level_temperatures(level_temperatures: Vec<f64>, decay_rate: f64) -> Self {
        Self::with_level_temperatures_and_rng(
            level_temperatures,
            decay_rate,
            DEFAULT_HILL_CLIMBING_TEMPERATURE,
            HardRegressionPolicy::TemperatureControlled,
        )
    }

    pub(crate) fn with_level_temperatures_and_seed(
        level_temperatures: Vec<f64>,
        decay_rate: f64,
        hill_climbing_temperature: f64,
        hard_regression_policy: HardRegressionPolicy,
        seed: u64,
    ) -> Self {
        Self::from_seed(
            TemperatureSeed::PerLevel(level_temperatures),
            decay_rate,
            hill_climbing_temperature,
            hard_regression_policy,
            SmallRng::seed_from_u64(seed),
        )
    }

    pub(crate) fn with_level_temperatures_and_rng(
        level_temperatures: Vec<f64>,
        decay_rate: f64,
        hill_climbing_temperature: f64,
        hard_regression_policy: HardRegressionPolicy,
    ) -> Self {
        Self::from_seed(
            TemperatureSeed::PerLevel(level_temperatures),
            decay_rate,
            hill_climbing_temperature,
            hard_regression_policy,
            SmallRng::from_rng(&mut rand::rng()),
        )
    }

    pub fn auto_calibrate(decay_rate: f64) -> Self {
        Self::with_calibration(
            decay_rate,
            DEFAULT_HILL_CLIMBING_TEMPERATURE,
            HardRegressionPolicy::TemperatureControlled,
            SimulatedAnnealingCalibration::default(),
        )
    }

    pub(crate) fn auto_calibrate_with_seed(decay_rate: f64, seed: u64) -> Self {
        Self::with_calibration_and_seed(
            decay_rate,
            DEFAULT_HILL_CLIMBING_TEMPERATURE,
            HardRegressionPolicy::TemperatureControlled,
            SimulatedAnnealingCalibration::default(),
            seed,
        )
    }

    pub(crate) fn with_calibration(
        decay_rate: f64,
        hill_climbing_temperature: f64,
        hard_regression_policy: HardRegressionPolicy,
        calibration: SimulatedAnnealingCalibration,
    ) -> Self {
        Self::from_seed(
            TemperatureSeed::Calibrated(calibration),
            decay_rate,
            hill_climbing_temperature,
            hard_regression_policy,
            SmallRng::from_rng(&mut rand::rng()),
        )
    }

    pub(crate) fn with_calibration_and_seed(
        decay_rate: f64,
        hill_climbing_temperature: f64,
        hard_regression_policy: HardRegressionPolicy,
        calibration: SimulatedAnnealingCalibration,
        seed: u64,
    ) -> Self {
        Self::from_seed(
            TemperatureSeed::Calibrated(calibration),
            decay_rate,
            hill_climbing_temperature,
            hard_regression_policy,
            SmallRng::seed_from_u64(seed),
        )
    }

    fn from_seed(
        temperature_seed: TemperatureSeed,
        decay_rate: f64,
        hill_climbing_temperature: f64,
        hard_regression_policy: HardRegressionPolicy,
        rng: SmallRng,
    ) -> Self {
        Self {
            temperature_seed,
            starting_temperatures: Vec::new(),
            current_temperatures: Vec::new(),
            decay_rate,
            hill_climbing_temperature,
            hard_regression_policy,
            calibration_state: None,
            rng,
            level_count: 0,
        }
    }

    pub(crate) fn current_temperature_for_level(&self, level: usize) -> f64 {
        self.current_temperatures[level]
    }

    fn install_temperatures(&mut self, temperatures: Vec<f64>) {
        debug_assert_eq!(temperatures.len(), self.level_count);
        self.starting_temperatures = temperatures.clone();
        self.current_temperatures = temperatures;
    }

    fn finalize_calibration_if_ready(&mut self) {
        let Some(state) = self.calibration_state.take() else {
            return;
        };
        if state.samples_seen >= state.config.sample_size {
            self.install_temperatures(state.temperatures());
        } else {
            self.calibration_state = Some(state);
        }
    }
}

impl Default for SimulatedAnnealingAcceptor {
    fn default() -> Self {
        Self::auto_calibrate(DEFAULT_DECAY_RATE)
    }
}

fn first_differing_level<Sc: Score>(last_step_score: &Sc, move_score: &Sc) -> Option<usize> {
    (0..Sc::levels_count())
        .find(|&level| last_step_score.level_number(level) != move_score.level_number(level))
}

fn worsening_delta_at_first_difference<Sc: Score>(
    last_step_score: &Sc,
    move_score: &Sc,
) -> Option<(usize, i64)> {
    let level = first_differing_level(last_step_score, move_score)?;
    let delta = move_score.level_number(level) - last_step_score.level_number(level);
    (delta < 0).then_some((level, delta))
}

fn assert_temperature(value: f64, field: &str) {
    assert!(
        value.is_finite() && value >= 0.0,
        "{field} must be finite and non-negative"
    );
}

pub(crate) fn assert_simulated_annealing_parameters(
    level_temperatures: Option<&[f64]>,
    level_count: usize,
    decay_rate: f64,
    hill_climbing_temperature: f64,
    calibration: Option<SimulatedAnnealingCalibration>,
) {
    assert!(
        decay_rate.is_finite() && decay_rate > 0.0 && decay_rate <= 1.0,
        "simulated_annealing decay_rate must be finite and in (0, 1]"
    );
    assert_temperature(
        hill_climbing_temperature,
        "simulated_annealing hill_climbing_temperature",
    );
    if let Some(temperatures) = level_temperatures {
        assert_eq!(
            temperatures.len(),
            level_count,
            "simulated_annealing level_temperatures length must match score level count"
        );
        for temperature in temperatures {
            assert_temperature(*temperature, "simulated_annealing level_temperatures");
        }
    }
    if let Some(calibration) = calibration {
        assert!(
            calibration.sample_size > 0,
            "simulated_annealing calibration sample_size must be greater than 0"
        );
        assert!(
            calibration.target_acceptance_probability.is_finite()
                && calibration.target_acceptance_probability > 0.0
                && calibration.target_acceptance_probability < 1.0,
            "simulated_annealing calibration target_acceptance_probability must be in (0, 1)"
        );
        assert_temperature(
            calibration.fallback_temperature,
            "simulated_annealing calibration fallback_temperature",
        );
    }
}

impl<S: PlanningSolution> Acceptor<S> for SimulatedAnnealingAcceptor
where
    S::Score: Score,
{
    fn is_accepted(
        &mut self,
        last_step_score: &S::Score,
        move_score: &S::Score,
        _move_signature: Option<&MoveTabuSignature>,
    ) -> bool {
        if *move_score >= *last_step_score {
            return true;
        }

        let Some((level, delta)) = worsening_delta_at_first_difference(last_step_score, move_score)
        else {
            return false;
        };

        if self.hard_regression_policy == HardRegressionPolicy::NeverAcceptHardRegression
            && S::Score::level_label(level) == ScoreLevel::Hard
        {
            return false;
        }

        if let Some(state) = self.calibration_state.as_mut() {
            let ready = state.record(level, delta.saturating_abs());
            if ready {
                self.finalize_calibration_if_ready();
            } else {
                return false;
            }
        }

        let temperature = self.current_temperatures[level];
        if temperature <= self.hill_climbing_temperature {
            return false;
        }

        let probability = ((delta as f64) / temperature).exp();
        self.rng.random::<f64>() < probability
    }

    fn phase_started(&mut self, _initial_score: &S::Score) {
        self.level_count = S::Score::levels_count();
        self.calibration_state = None;
        match self.temperature_seed.clone() {
            TemperatureSeed::Single(temperature) => {
                let temperatures = vec![temperature; self.level_count];
                assert_simulated_annealing_parameters(
                    Some(&temperatures),
                    self.level_count,
                    self.decay_rate,
                    self.hill_climbing_temperature,
                    None,
                );
                self.install_temperatures(temperatures);
            }
            TemperatureSeed::PerLevel(temperatures) => {
                assert_simulated_annealing_parameters(
                    Some(&temperatures),
                    self.level_count,
                    self.decay_rate,
                    self.hill_climbing_temperature,
                    None,
                );
                self.install_temperatures(temperatures);
            }
            TemperatureSeed::Calibrated(calibration) => {
                assert_simulated_annealing_parameters(
                    None,
                    self.level_count,
                    self.decay_rate,
                    self.hill_climbing_temperature,
                    Some(calibration),
                );
                self.install_temperatures(vec![0.0; self.level_count]);
                self.calibration_state = Some(CalibrationState::new(self.level_count, calibration));
            }
        }
    }

    fn step_ended(
        &mut self,
        _step_score: &S::Score,
        _accepted_move_signature: Option<&MoveTabuSignature>,
    ) {
        if self.calibration_state.is_some() {
            return;
        }
        for temperature in &mut self.current_temperatures {
            *temperature *= self.decay_rate;
            if *temperature < self.hill_climbing_temperature {
                *temperature = self.hill_climbing_temperature;
            }
        }
    }
}

#[cfg(test)]
mod tests;
