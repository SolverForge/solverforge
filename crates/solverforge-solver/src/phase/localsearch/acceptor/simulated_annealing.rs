//! Simulated annealing acceptor with true Boltzmann distribution.

use std::fmt::Debug;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use super::Acceptor;

/// Simulated annealing acceptor using the Boltzmann distribution.
///
/// Accepts improving moves unconditionally. For worsening moves, accepts with
/// probability `exp(-delta / T)` where `delta` is the score degradation and
/// `T` is the current temperature.
///
/// Temperature decays geometrically each step: `T *= decay_rate`.
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::localsearch::SimulatedAnnealingAcceptor;
///
/// // High initial temperature (explores broadly), slow cooling
/// let acceptor = SimulatedAnnealingAcceptor::new(1.0, 0.9999);
/// ```
#[derive(Debug)]
pub struct SimulatedAnnealingAcceptor {
    /// Initial temperature.
    starting_temperature: f64,
    /// Current temperature.
    current_temperature: f64,
    /// Temperature decay rate per step.
    decay_rate: f64,
    /// High-quality RNG for acceptance decisions.
    rng: StdRng,
    /// Number of score levels, cached after phase_started.
    level_count: usize,
}

impl SimulatedAnnealingAcceptor {
    /// Creates a new simulated annealing acceptor.
    ///
    /// # Arguments
    /// * `starting_temperature` - Initial temperature (higher = more exploration).
    ///   Calibrate to ~20% of the initial hard score magnitude for best results.
    /// * `decay_rate` - Multiplicative decay per step (e.g., 0.9999 for 30s runs
    ///   at ~10k steps/s gives final T ≈ 0.05 * starting T).
    pub fn new(starting_temperature: f64, decay_rate: f64) -> Self {
        Self {
            starting_temperature,
            current_temperature: starting_temperature,
            decay_rate,
            rng: StdRng::from_os_rng(),
            level_count: 0,
        }
    }

    /// Creates a new SA acceptor with a fixed seed for reproducibility.
    pub fn with_seed(starting_temperature: f64, decay_rate: f64, seed: u64) -> Self {
        Self {
            starting_temperature,
            current_temperature: starting_temperature,
            decay_rate,
            rng: StdRng::seed_from_u64(seed),
            level_count: 0,
        }
    }

    /// Auto-calibrates starting temperature from the initial score.
    ///
    /// Sets temperature to 20% of the absolute initial score magnitude,
    /// ensuring ~80% acceptance probability for moves with delta = |initial_score|.
    pub fn auto_calibrate(decay_rate: f64) -> Self {
        Self {
            starting_temperature: 0.0, // Will be set in phase_started
            current_temperature: 0.0,
            decay_rate,
            rng: StdRng::from_os_rng(),
            level_count: 0,
        }
    }
}

impl Default for SimulatedAnnealingAcceptor {
    fn default() -> Self {
        // Auto-calibrate with a decay rate tuned for ~300k steps in 30s.
        // At 300k steps, decay_rate^300000 ≈ 0.01 when decay_rate ≈ 0.999985.
        Self::auto_calibrate(0.999985)
    }
}

/// Converts a multi-level score difference to a single scalar for SA.
///
/// Hard levels are weighted exponentially more than soft levels so that
/// hard constraint improvements always dominate the acceptance probability.
///
/// NOTE: This is only used by `auto_calibrate` during `phase_started`.
/// The hot-path `is_accepted` uses `Score::to_scalar()` directly (zero alloc).
fn score_delta_to_scalar(levels: &[i64]) -> f64 {
    if levels.is_empty() {
        return 0.0;
    }
    if levels.len() == 1 {
        return levels[0] as f64;
    }
    let n = levels.len();
    let mut scalar = 0.0f64;
    for (i, &level) in levels.iter().enumerate() {
        let weight = 10.0f64.powi(6 * (n - 1 - i) as i32);
        scalar += level as f64 * weight;
    }
    scalar
}

impl<S: PlanningSolution> Acceptor<S> for SimulatedAnnealingAcceptor
where
    S::Score: Score,
{
    fn is_accepted(&mut self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Always accept improving or equal moves
        if *move_score >= *last_step_score {
            return true;
        }

        if self.current_temperature <= 0.0 {
            return false;
        }

        // Compute score difference: move_score - last_step_score (negative for worsening).
        // Uses Score::to_scalar() directly — no Vec allocation.
        let delta = move_score.to_scalar() - last_step_score.to_scalar();

        // delta is negative (worsening move). Acceptance probability = exp(delta / T).
        let probability = (delta / self.current_temperature).exp();
        let roll: f64 = self.rng.random();

        roll < probability
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        self.level_count = S::Score::levels_count();

        // Auto-calibrate temperature if it was set to 0 (from auto_calibrate())
        if self.starting_temperature == 0.0 {
            let levels = initial_score.to_level_numbers();
            let magnitude = score_delta_to_scalar(&levels).abs();
            // Set to 2% of score magnitude. For HardSoftScore, hard levels are
            // weighted by 10^6, so this gives enough room to accept occasional
            // hard-worsening moves at the start while cooling to pure hill-climbing.
            self.starting_temperature = if magnitude > 0.0 {
                magnitude * 0.02
            } else {
                1.0
            };
        }

        self.current_temperature = self.starting_temperature;
    }

    fn step_ended(&mut self, _step_score: &S::Score) {
        self.current_temperature *= self.decay_rate;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::domain::PlanningSolution;
    use solverforge_core::score::{HardSoftScore, SimpleScore};

    #[derive(Clone, Debug)]
    struct SimpleSol {
        score: Option<SimpleScore>,
    }
    impl PlanningSolution for SimpleSol {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    #[derive(Clone, Debug)]
    struct HardSoftSol {
        score: Option<HardSoftScore>,
    }
    impl PlanningSolution for HardSoftSol {
        type Score = HardSoftScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    #[test]
    fn accepts_improving_moves() {
        let mut acceptor = SimulatedAnnealingAcceptor::with_seed(1000.0, 0.99, 42);
        let last = SimpleScore::of(-10);
        let better = SimpleScore::of(-5);
        assert!(Acceptor::<SimpleSol>::is_accepted(
            &mut acceptor,
            &last,
            &better
        ));
    }

    #[test]
    fn accepts_equal_moves() {
        let mut acceptor = SimulatedAnnealingAcceptor::with_seed(1000.0, 0.99, 42);
        let score = SimpleScore::of(-10);
        assert!(Acceptor::<SimpleSol>::is_accepted(
            &mut acceptor,
            &score,
            &score
        ));
    }

    #[test]
    fn rejects_at_zero_temperature() {
        let mut acceptor = SimulatedAnnealingAcceptor::with_seed(0.0, 0.99, 42);
        acceptor.current_temperature = 0.0;
        let last = SimpleScore::of(-10);
        let worse = SimpleScore::of(-20);
        assert!(!Acceptor::<SimpleSol>::is_accepted(
            &mut acceptor,
            &last,
            &worse
        ));
    }

    #[test]
    fn high_temperature_accepts_most() {
        let mut acceptor = SimulatedAnnealingAcceptor::with_seed(1_000_000.0, 0.99, 42);
        let last = SimpleScore::of(-10);
        let worse = SimpleScore::of(-11);
        let mut accepted = 0;
        for _ in 0..100 {
            if Acceptor::<SimpleSol>::is_accepted(&mut acceptor, &last, &worse) {
                accepted += 1;
            }
        }
        assert!(accepted > 90);
    }

    #[test]
    fn low_temperature_rejects_most() {
        let mut acceptor = SimulatedAnnealingAcceptor::with_seed(0.001, 0.99, 42);
        let last = SimpleScore::of(-10);
        let worse = SimpleScore::of(-20);
        let mut accepted = 0;
        for _ in 0..100 {
            if Acceptor::<SimpleSol>::is_accepted(&mut acceptor, &last, &worse) {
                accepted += 1;
            }
        }
        assert!(accepted < 5);
    }

    #[test]
    fn temperature_decays_each_step() {
        let mut acceptor = SimulatedAnnealingAcceptor::with_seed(100.0, 0.5, 42);
        let score = SimpleScore::of(0);
        Acceptor::<SimpleSol>::phase_started(&mut acceptor, &score);
        assert!((acceptor.current_temperature - 100.0).abs() < f64::EPSILON);
        Acceptor::<SimpleSol>::step_ended(&mut acceptor, &score);
        assert!((acceptor.current_temperature - 50.0).abs() < f64::EPSILON);
        Acceptor::<SimpleSol>::step_ended(&mut acceptor, &score);
        assert!((acceptor.current_temperature - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn auto_calibrate_sets_temperature_from_initial_score() {
        let mut acceptor = SimulatedAnnealingAcceptor::auto_calibrate(0.999);
        let initial = HardSoftScore::of(-576, 0);
        Acceptor::<HardSoftSol>::phase_started(&mut acceptor, &initial);
        // Temperature should be ~2% of 576 * 1_000_000 = 11_520_000
        assert!(acceptor.current_temperature > 10_000_000.0);
        assert!(acceptor.current_temperature < 20_000_000.0);
    }

    #[test]
    fn score_delta_to_scalar_simple() {
        assert!((score_delta_to_scalar(&[-5]) - -5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn score_delta_to_scalar_hard_soft() {
        let scalar = score_delta_to_scalar(&[-1, -50]);
        assert!((scalar - -1_000_050.0).abs() < f64::EPSILON);
    }
}
