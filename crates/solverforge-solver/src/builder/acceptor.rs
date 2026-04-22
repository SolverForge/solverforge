// Acceptor builder and `AnyAcceptor` enum.

use std::fmt::Debug;

use solverforge_config::{AcceptorConfig, TabuSearchConfig};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};

use crate::heuristic::r#move::MoveTabuSignature;
use crate::phase::localsearch::{
    Acceptor, DiversifiedLateAcceptanceAcceptor, GreatDelugeAcceptor, HillClimbingAcceptor,
    LateAcceptanceAcceptor, SimulatedAnnealingAcceptor, StepCountingHillClimbingAcceptor,
    TabuSearchAcceptor, TabuSearchPolicy,
};

/* A concrete enum over all built-in acceptor types.

Returned by [`AcceptorBuilder::build`] to avoid `Box<dyn Acceptor<S>>`.
Dispatches to the inner acceptor via `match` — fully monomorphized.
*/
#[allow(clippy::large_enum_variant)]
pub enum AnyAcceptor<S: PlanningSolution> {
    // Hill climbing acceptor.
    HillClimbing(HillClimbingAcceptor),
    // Step counting hill climbing acceptor.
    StepCountingHillClimbing(StepCountingHillClimbingAcceptor<S>),
    // Tabu search acceptor.
    TabuSearch(TabuSearchAcceptor<S>),
    // Simulated annealing acceptor.
    SimulatedAnnealing(SimulatedAnnealingAcceptor),
    // Late acceptance acceptor.
    LateAcceptance(LateAcceptanceAcceptor<S>),
    // Diversified late acceptance acceptor.
    DiversifiedLateAcceptance(DiversifiedLateAcceptanceAcceptor<S>),
    // Great deluge acceptor.
    GreatDeluge(GreatDelugeAcceptor<S>),
}

impl<S: PlanningSolution> Debug for AnyAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HillClimbing(a) => write!(f, "AnyAcceptor::HillClimbing({a:?})"),
            Self::StepCountingHillClimbing(a) => {
                write!(f, "AnyAcceptor::StepCountingHillClimbing({a:?})")
            }
            Self::TabuSearch(a) => write!(f, "AnyAcceptor::TabuSearch({a:?})"),
            Self::SimulatedAnnealing(a) => write!(f, "AnyAcceptor::SimulatedAnnealing({a:?})"),
            Self::LateAcceptance(a) => write!(f, "AnyAcceptor::LateAcceptance({a:?})"),
            Self::DiversifiedLateAcceptance(a) => {
                write!(f, "AnyAcceptor::DiversifiedLateAcceptance({a:?})")
            }
            Self::GreatDeluge(a) => write!(f, "AnyAcceptor::GreatDeluge({a:?})"),
        }
    }
}

impl<S: PlanningSolution> Clone for AnyAcceptor<S>
where
    S::Score: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::HillClimbing(a) => Self::HillClimbing(a.clone()),
            Self::StepCountingHillClimbing(a) => Self::StepCountingHillClimbing(a.clone()),
            Self::TabuSearch(a) => Self::TabuSearch(a.clone()),
            Self::SimulatedAnnealing(a) => Self::SimulatedAnnealing(a.clone()),
            Self::LateAcceptance(a) => Self::LateAcceptance(a.clone()),
            Self::DiversifiedLateAcceptance(a) => Self::DiversifiedLateAcceptance(a.clone()),
            Self::GreatDeluge(a) => Self::GreatDeluge(a.clone()),
        }
    }
}

impl<S: PlanningSolution> Acceptor<S> for AnyAcceptor<S>
where
    S::Score: Score,
{
    fn requires_move_signatures(&self) -> bool {
        match self {
            Self::HillClimbing(a) => Acceptor::<S>::requires_move_signatures(a),
            Self::StepCountingHillClimbing(a) => Acceptor::<S>::requires_move_signatures(a),
            Self::TabuSearch(a) => Acceptor::<S>::requires_move_signatures(a),
            Self::SimulatedAnnealing(a) => Acceptor::<S>::requires_move_signatures(a),
            Self::LateAcceptance(a) => Acceptor::<S>::requires_move_signatures(a),
            Self::DiversifiedLateAcceptance(a) => Acceptor::<S>::requires_move_signatures(a),
            Self::GreatDeluge(a) => Acceptor::<S>::requires_move_signatures(a),
        }
    }

    fn is_accepted(
        &mut self,
        last_step_score: &S::Score,
        move_score: &S::Score,
        move_signature: Option<&MoveTabuSignature>,
    ) -> bool {
        match self {
            Self::HillClimbing(a) => {
                Acceptor::<S>::is_accepted(a, last_step_score, move_score, move_signature)
            }
            Self::StepCountingHillClimbing(a) => {
                Acceptor::<S>::is_accepted(a, last_step_score, move_score, move_signature)
            }
            Self::TabuSearch(a) => {
                Acceptor::<S>::is_accepted(a, last_step_score, move_score, move_signature)
            }
            Self::SimulatedAnnealing(a) => {
                Acceptor::<S>::is_accepted(a, last_step_score, move_score, move_signature)
            }
            Self::LateAcceptance(a) => {
                Acceptor::<S>::is_accepted(a, last_step_score, move_score, move_signature)
            }
            Self::DiversifiedLateAcceptance(a) => {
                Acceptor::<S>::is_accepted(a, last_step_score, move_score, move_signature)
            }
            Self::GreatDeluge(a) => {
                Acceptor::<S>::is_accepted(a, last_step_score, move_score, move_signature)
            }
        }
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        match self {
            Self::HillClimbing(a) => Acceptor::<S>::phase_started(a, initial_score),
            Self::StepCountingHillClimbing(a) => Acceptor::<S>::phase_started(a, initial_score),
            Self::TabuSearch(a) => Acceptor::<S>::phase_started(a, initial_score),
            Self::SimulatedAnnealing(a) => Acceptor::<S>::phase_started(a, initial_score),
            Self::LateAcceptance(a) => Acceptor::<S>::phase_started(a, initial_score),
            Self::DiversifiedLateAcceptance(a) => Acceptor::<S>::phase_started(a, initial_score),
            Self::GreatDeluge(a) => Acceptor::<S>::phase_started(a, initial_score),
        }
    }

    fn phase_ended(&mut self) {
        match self {
            Self::HillClimbing(a) => Acceptor::<S>::phase_ended(a),
            Self::StepCountingHillClimbing(a) => Acceptor::<S>::phase_ended(a),
            Self::TabuSearch(a) => Acceptor::<S>::phase_ended(a),
            Self::SimulatedAnnealing(a) => Acceptor::<S>::phase_ended(a),
            Self::LateAcceptance(a) => Acceptor::<S>::phase_ended(a),
            Self::DiversifiedLateAcceptance(a) => Acceptor::<S>::phase_ended(a),
            Self::GreatDeluge(a) => Acceptor::<S>::phase_ended(a),
        }
    }

    fn step_started(&mut self) {
        match self {
            Self::HillClimbing(a) => Acceptor::<S>::step_started(a),
            Self::StepCountingHillClimbing(a) => Acceptor::<S>::step_started(a),
            Self::TabuSearch(a) => Acceptor::<S>::step_started(a),
            Self::SimulatedAnnealing(a) => Acceptor::<S>::step_started(a),
            Self::LateAcceptance(a) => Acceptor::<S>::step_started(a),
            Self::DiversifiedLateAcceptance(a) => Acceptor::<S>::step_started(a),
            Self::GreatDeluge(a) => Acceptor::<S>::step_started(a),
        }
    }

    fn step_ended(
        &mut self,
        step_score: &S::Score,
        accepted_move_signature: Option<&MoveTabuSignature>,
    ) {
        match self {
            Self::HillClimbing(a) => {
                Acceptor::<S>::step_ended(a, step_score, accepted_move_signature)
            }
            Self::StepCountingHillClimbing(a) => {
                Acceptor::<S>::step_ended(a, step_score, accepted_move_signature)
            }
            Self::TabuSearch(a) => {
                Acceptor::<S>::step_ended(a, step_score, accepted_move_signature)
            }
            Self::SimulatedAnnealing(a) => {
                Acceptor::<S>::step_ended(a, step_score, accepted_move_signature)
            }
            Self::LateAcceptance(a) => {
                Acceptor::<S>::step_ended(a, step_score, accepted_move_signature)
            }
            Self::DiversifiedLateAcceptance(a) => {
                Acceptor::<S>::step_ended(a, step_score, accepted_move_signature)
            }
            Self::GreatDeluge(a) => {
                Acceptor::<S>::step_ended(a, step_score, accepted_move_signature)
            }
        }
    }
}

/// Builder for constructing acceptors from configuration.
pub struct AcceptorBuilder;

impl AcceptorBuilder {
    /// Builds a concrete [`AnyAcceptor`] from configuration.
    pub fn build<S: PlanningSolution>(config: &AcceptorConfig) -> AnyAcceptor<S>
    where
        S::Score: Score + ParseableScore,
    {
        Self::build_with_seed(config, None)
    }

    /// Builds a concrete [`AnyAcceptor`] from configuration with an optional deterministic seed.
    pub fn build_with_seed<S: PlanningSolution>(
        config: &AcceptorConfig,
        random_seed: Option<u64>,
    ) -> AnyAcceptor<S>
    where
        S::Score: Score + ParseableScore,
    {
        match config {
            AcceptorConfig::HillClimbing => AnyAcceptor::HillClimbing(HillClimbingAcceptor::new()),

            AcceptorConfig::StepCountingHillClimbing(step_counting_config) => {
                AnyAcceptor::StepCountingHillClimbing(StepCountingHillClimbingAcceptor::new(
                    step_counting_config.step_count_limit.unwrap_or(100),
                ))
            }

            AcceptorConfig::TabuSearch(tabu_config) => {
                AnyAcceptor::TabuSearch(TabuSearchAcceptor::<S>::new(
                    normalize_tabu_search_policy(tabu_config),
                ))
            }

            AcceptorConfig::SimulatedAnnealing(sa_config) => {
                let starting_temp = sa_config.starting_temperature.as_ref().map(|s| {
                    s.parse::<f64>()
                        .ok()
                        .or_else(|| S::Score::parse(s).ok().map(|score| score.to_scalar().abs()))
                        .unwrap_or_else(|| {
                            panic!("Invalid starting_temperature '{}': expected scalar or score string", s)
                        })
                });
                let decay_rate = sa_config.decay_rate.unwrap_or(0.999985);
                AnyAcceptor::SimulatedAnnealing(match (starting_temp, random_seed) {
                    (Some(temp), Some(seed)) => {
                        SimulatedAnnealingAcceptor::with_seed(temp, decay_rate, seed)
                    }
                    (Some(temp), None) => SimulatedAnnealingAcceptor::new(temp, decay_rate),
                    (None, Some(seed)) => {
                        SimulatedAnnealingAcceptor::auto_calibrate_with_seed(decay_rate, seed)
                    }
                    (None, None) => SimulatedAnnealingAcceptor::auto_calibrate(decay_rate),
                })
            }

            AcceptorConfig::LateAcceptance(la_config) => {
                let size = la_config.late_acceptance_size.unwrap_or(400);
                AnyAcceptor::LateAcceptance(LateAcceptanceAcceptor::<S>::new(size))
            }

            AcceptorConfig::DiversifiedLateAcceptance(dla_config) => {
                let size = dla_config.late_acceptance_size.unwrap_or(400);
                let tolerance = dla_config.tolerance.unwrap_or(0.01);
                AnyAcceptor::DiversifiedLateAcceptance(DiversifiedLateAcceptanceAcceptor::<S>::new(
                    size, tolerance,
                ))
            }

            AcceptorConfig::GreatDeluge(gd_config) => {
                let rain_speed = gd_config.water_level_increase_ratio.unwrap_or(0.001);
                AnyAcceptor::GreatDeluge(GreatDelugeAcceptor::<S>::new(rain_speed))
            }
        }
    }

    pub fn hill_climbing<S: PlanningSolution>() -> HillClimbingAcceptor {
        HillClimbingAcceptor::new()
    }

    pub fn tabu_search<S: PlanningSolution>(tabu_size: usize) -> TabuSearchAcceptor<S> {
        TabuSearchAcceptor::<S>::new(TabuSearchPolicy::move_only(tabu_size))
    }

    pub fn simulated_annealing(starting_temp: f64, decay_rate: f64) -> SimulatedAnnealingAcceptor {
        SimulatedAnnealingAcceptor::new(starting_temp, decay_rate)
    }

    pub fn late_acceptance<S: PlanningSolution>(size: usize) -> LateAcceptanceAcceptor<S> {
        LateAcceptanceAcceptor::<S>::new(size)
    }
}

fn validate_tabu_size(field_name: &str, value: Option<usize>) -> Option<usize> {
    match value {
        Some(0) => panic!("tabu_search field `{field_name}` must be greater than 0"),
        _ => value,
    }
}

fn normalize_tabu_search_policy(config: &TabuSearchConfig) -> TabuSearchPolicy {
    let entity_tabu_size = validate_tabu_size("entity_tabu_size", config.entity_tabu_size);
    let value_tabu_size = validate_tabu_size("value_tabu_size", config.value_tabu_size);
    let move_tabu_size = validate_tabu_size("move_tabu_size", config.move_tabu_size);
    let undo_move_tabu_size =
        validate_tabu_size("undo_move_tabu_size", config.undo_move_tabu_size);
    let aspiration_enabled = config.aspiration_enabled.unwrap_or(true);

    if entity_tabu_size.is_none()
        && value_tabu_size.is_none()
        && move_tabu_size.is_none()
        && undo_move_tabu_size.is_none()
    {
        return TabuSearchPolicy {
            aspiration_enabled,
            ..TabuSearchPolicy::move_only(10)
        };
    }

    TabuSearchPolicy {
        entity_tabu_size,
        value_tabu_size,
        move_tabu_size,
        undo_move_tabu_size,
        aspiration_enabled,
    }
}

#[cfg(test)]
#[path = "acceptor_tests.rs"]
mod tests;
