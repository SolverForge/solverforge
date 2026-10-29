use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;
use solverforge::{AcceptorConfig, MoveSelectorConfig, PhaseConfig, SolverConfig};
use solverforge_config::{
    ConstructionHeuristicConfig, ListChangeMoveConfig, LocalSearchConfig, TabuSearchConfig,
    TerminationConfig,
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
        ConfigCase::ZeroTabuSize | ConfigCase::InvalidSolverScoreLimit | ConfigCase::Valid => {
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
            PhaseConfig::ConstructionHeuristic(ConstructionHeuristicConfig::default()),
            PhaseConfig::LocalSearch(local_search),
        ],
        ..SolverConfig::default()
    }
}
