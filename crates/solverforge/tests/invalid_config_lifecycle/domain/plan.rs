use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;
use solverforge::{AcceptorConfig, MoveSelectorConfig, PhaseConfig, SolverConfig};
use solverforge_config::{
    ChangeMoveConfig, ConstructionHeuristicConfig, ListChangeMoveConfig, LocalSearchConfig,
    PillarChangeMoveConfig, PillarSwapMoveConfig, TabuSearchConfig, TerminationConfig,
};

use super::{Resource, Task};

/// Which solver configuration the config callback hands to the solve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigCase {
    /// Construction, then local search with `entity_tabu_size = 0`.
    ZeroTabuSize,
    /// Construction, then a list selector on this scalar-only model.
    ListSelectorOnScalarModel,
    /// A solver `best_score_limit` that is not a `HardSoftScore`.
    InvalidSolverScoreLimit,
    /// A local-search phase `best_score_limit` that is not a `HardSoftScore`.
    InvalidPhaseScoreLimit,
    /// Construction with `value_candidate_limit = 0`.
    ZeroConstructionValueCandidateLimit,
    /// Construction, then a change selector with `value_candidate_limit = 0`.
    ZeroChangeValueCandidateLimit,
    /// Construction, then pillar change with sub-pillar sizes `5/2`.
    PillarChangeMinimumAboveMaximum,
    /// Construction, then pillar swap with sub-pillar sizes `5/2`.
    PillarSwapMinimumAboveMaximum,
    /// Construction, then pillar change with `0/0` (full pillars only).
    FullPillarChange,
    /// Construction, then a valid tabu search.
    Valid,
}

#[planning_solution(constraints = "define_constraints", config = "config_for_case")]
pub struct Plan {
    #[problem_fact_collection]
    pub resources: Vec<Resource>,

    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,

    pub case: ConfigCase,
}

fn define_constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::tasks())
        .filter(|task: &Task| task.resource_idx.is_none())
        .penalize(HardSoftScore::of(0, 1))
        .named("unassigned task"),)
}

fn config_for_case(plan: &Plan, _: SolverConfig) -> SolverConfig {
    let local_search = match plan.case {
        ConfigCase::ZeroTabuSize
        | ConfigCase::InvalidSolverScoreLimit
        | ConfigCase::ZeroConstructionValueCandidateLimit
        | ConfigCase::Valid => {
            LocalSearchConfig {
            acceptor: Some(AcceptorConfig::TabuSearch(TabuSearchConfig {
                    entity_tabu_size: Some(if plan.case == ConfigCase::ZeroTabuSize {
                        0
                    } else {
                        2
                    }),
                    ..TabuSearchConfig::default()
                })),
                ..LocalSearchConfig::default()
            }
        }
        ConfigCase::InvalidPhaseScoreLimit => LocalSearchConfig {
            termination: Some(TerminationConfig {
                best_score_limit: Some("0hard/-5sotf".to_string()),
                ..TerminationConfig::default()
            }),
            ..LocalSearchConfig::default()
        },
        ConfigCase::ZeroChangeValueCandidateLimit => {
            selector(MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                value_candidate_limit: Some(0),
                ..ChangeMoveConfig::default()
            }))
        }
        ConfigCase::PillarChangeMinimumAboveMaximum => selector(
            MoveSelectorConfig::PillarChangeMoveSelector(PillarChangeMoveConfig {
                minimum_sub_pillar_size: 5,
                maximum_sub_pillar_size: 2,
                ..PillarChangeMoveConfig::default()
            }),
        ),
        ConfigCase::PillarSwapMinimumAboveMaximum => selector(
            MoveSelectorConfig::PillarSwapMoveSelector(PillarSwapMoveConfig {
                minimum_sub_pillar_size: 5,
                maximum_sub_pillar_size: 2,
                ..PillarSwapMoveConfig::default()
            }),
        ),
        ConfigCase::FullPillarChange => selector(MoveSelectorConfig::PillarChangeMoveSelector(
            PillarChangeMoveConfig::default(),
        )),
        ConfigCase::ListSelectorOnScalarModel => LocalSearchConfig {
            move_selector: Some(MoveSelectorConfig::ListChangeMoveSelector(
                ListChangeMoveConfig::default(),
            )),
            ..LocalSearchConfig::default()
        },
    };
    SolverConfig {
        random_seed: Some(7),
        termination: Some(TerminationConfig {
            step_count_limit: Some(20),
            best_score_limit: (plan.case == ConfigCase::InvalidSolverScoreLimit)
                .then(|| "not-a-score".to_string()),
            ..TerminationConfig::default()
        }),
        phases: vec![
            PhaseConfig::ConstructionHeuristic(ConstructionHeuristicConfig {
                value_candidate_limit: (plan.case
                    == ConfigCase::ZeroConstructionValueCandidateLimit)
                    .then_some(0),
                ..ConstructionHeuristicConfig::default()
            }),
            PhaseConfig::LocalSearch(local_search),
        ],
        ..SolverConfig::default()
    }
}

fn selector(move_selector: MoveSelectorConfig) -> LocalSearchConfig {
    LocalSearchConfig {
        move_selector: Some(move_selector),
        ..LocalSearchConfig::default()
    }
}
