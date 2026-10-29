//! Acceptor and forager settings checked against one score type when the solver
//! configuration is compiled, so building them when a phase starts cannot fail.

use std::fmt;
use std::marker::PhantomData;

use solverforge_config::{
    AcceptorConfig, ForagerConfig, ScoreTieBreak, SimulatedAnnealingCalibrationConfig,
    SimulatedAnnealingConfig, TabuSearchConfig,
};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};

use crate::phase::localsearch::SimulatedAnnealingCalibration;

use super::acceptor::{build_validated_acceptor, AnyAcceptor};
use super::forager::{build_validated_forager, AnyForager};

/// An acceptor or forager setting outside its valid domain.
///
/// `field` is the configuration key below the acceptor or forager table, for
/// example `entity_tabu_size` or `calibration.sample_size`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConfigValueError {
    field: &'static str,
    message: String,
}

impl ConfigValueError {
    fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }

    /// The configuration key that holds the invalid value.
    pub(crate) fn field(&self) -> &'static str {
        self.field
    }

    /// Why the value is invalid.
    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ConfigValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` {}", self.field, self.message)
    }
}

impl std::error::Error for ConfigValueError {}

/// An acceptor configuration whose values are valid for the score type of `S`.
pub(crate) struct CompiledAcceptor<S> {
    config: AcceptorConfig,
    _score: PhantomData<fn() -> S>,
}

impl<S> Clone for CompiledAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            _score: PhantomData,
        }
    }
}

impl<S> fmt::Debug for CompiledAcceptor<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CompiledAcceptor")
            .field(&self.config)
            .finish()
    }
}

impl<S> CompiledAcceptor<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    pub(crate) fn compile(config: &AcceptorConfig) -> Result<Self, ConfigValueError> {
        validate_acceptor(config, S::Score::levels_count())?;
        Ok(Self {
            config: config.clone(),
            _score: PhantomData,
        })
    }

    pub(crate) fn is_tabu_search(&self) -> bool {
        matches!(self.config, AcceptorConfig::TabuSearch(_))
    }

    pub(crate) fn build(&self, random_seed: Option<u64>) -> AnyAcceptor<S>
    where
        S::Score: ParseableScore,
    {
        build_validated_acceptor(&self.config, random_seed)
    }
}

/// A forager configuration whose values are valid.
#[derive(Clone, Debug)]
pub(crate) struct CompiledForager {
    config: ForagerConfig,
}

impl CompiledForager {
    pub(crate) fn compile(config: &ForagerConfig) -> Result<Self, ConfigValueError> {
        if let ForagerConfig::AcceptedCount(accepted) = config {
            positive("limit", accepted.limit)?;
        }
        Ok(Self {
            config: config.clone(),
        })
    }

    pub(crate) fn build<S>(&self, score_tie_break: ScoreTieBreak) -> AnyForager<S>
    where
        S: PlanningSolution,
        S::Score: Score,
    {
        build_validated_forager(&self.config, score_tie_break)
    }
}

fn validate_acceptor(config: &AcceptorConfig, level_count: usize) -> Result<(), ConfigValueError> {
    match config {
        AcceptorConfig::HillClimbing {} | AcceptorConfig::StepCountingHillClimbing(_) => Ok(()),
        AcceptorConfig::TabuSearch(tabu) => validate_tabu_search(tabu),
        AcceptorConfig::SimulatedAnnealing(annealing) => {
            validate_simulated_annealing(annealing, level_count)
        }
        AcceptorConfig::LateAcceptance(late) => {
            positive("late_acceptance_size", late.late_acceptance_size)
        }
        AcceptorConfig::DiversifiedLateAcceptance(late) => {
            positive("late_acceptance_size", late.late_acceptance_size)
        }
        AcceptorConfig::GreatDeluge(_) => Ok(()),
    }
}

fn validate_tabu_search(config: &TabuSearchConfig) -> Result<(), ConfigValueError> {
    positive("entity_tabu_size", config.entity_tabu_size)?;
    positive("value_tabu_size", config.value_tabu_size)?;
    positive("move_tabu_size", config.move_tabu_size)?;
    positive("undo_move_tabu_size", config.undo_move_tabu_size)
}

fn validate_simulated_annealing(
    config: &SimulatedAnnealingConfig,
    level_count: usize,
) -> Result<(), ConfigValueError> {
    if let Some(decay_rate) = config.decay_rate {
        if !(decay_rate.is_finite() && decay_rate > 0.0 && decay_rate <= 1.0) {
            return Err(ConfigValueError::new(
                "decay_rate",
                format!("must be finite and in (0, 1], found {decay_rate}"),
            ));
        }
    }
    non_negative(
        "hill_climbing_temperature",
        config.hill_climbing_temperature,
    )?;
    match &config.level_temperatures {
        Some(temperatures) => {
            if temperatures.len() != level_count {
                return Err(ConfigValueError::new(
                    "level_temperatures",
                    format!(
                        "must have one temperature per score level ({level_count}), found {}",
                        temperatures.len()
                    ),
                ));
            }
            for &temperature in temperatures {
                non_negative("level_temperatures", Some(temperature))?;
            }
            Ok(())
        }
        None => validate_calibration(config.calibration.as_ref()),
    }
}

fn validate_calibration(
    config: Option<&SimulatedAnnealingCalibrationConfig>,
) -> Result<(), ConfigValueError> {
    let Some(config) = config else {
        return Ok(());
    };
    positive("calibration.sample_size", config.sample_size)?;
    if let Some(probability) = config.target_acceptance_probability {
        if !(probability.is_finite() && probability > 0.0 && probability < 1.0) {
            return Err(ConfigValueError::new(
                "calibration.target_acceptance_probability",
                format!("must be in (0, 1), found {probability}"),
            ));
        }
    }
    non_negative(
        "calibration.fallback_temperature",
        config.fallback_temperature,
    )
}

fn positive(field: &'static str, value: Option<usize>) -> Result<(), ConfigValueError> {
    match value {
        Some(0) => Err(ConfigValueError::new(field, "must be greater than 0")),
        _ => Ok(()),
    }
}

fn non_negative(field: &'static str, value: Option<f64>) -> Result<(), ConfigValueError> {
    match value {
        Some(value) if !(value.is_finite() && value >= 0.0) => Err(ConfigValueError::new(
            field,
            format!("must be finite and non-negative, found {value}"),
        )),
        _ => Ok(()),
    }
}

/// The calibration a validated simulated-annealing configuration runs with.
pub(super) fn resolved_calibration(
    config: Option<&SimulatedAnnealingCalibrationConfig>,
) -> SimulatedAnnealingCalibration {
    let defaults = SimulatedAnnealingCalibration::default();
    SimulatedAnnealingCalibration {
        sample_size: config
            .and_then(|config| config.sample_size)
            .unwrap_or(defaults.sample_size),
        target_acceptance_probability: config
            .and_then(|config| config.target_acceptance_probability)
            .unwrap_or(defaults.target_acceptance_probability),
        fallback_temperature: config
            .and_then(|config| config.fallback_temperature)
            .unwrap_or(defaults.fallback_temperature),
    }
}
