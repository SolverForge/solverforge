use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;
use solverforge::SolverConfig;

use super::{Resource, Task};

#[planning_solution(
    constraints = "define_constraints",
    constraint_provider = "relaxed_constraints",
    config = "director_config"
)]
pub struct Plan {
    #[problem_fact_collection]
    pub resources: Vec<Resource>,

    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,

    /// The `[score_director]` table a retained solve of this plan runs with.
    pub score_director_toml: &'static str,
}

impl Plan {
    pub fn new(score_director_toml: &'static str) -> Self {
        Self {
            resources: (0..3).map(|id| Resource { id }).collect(),
            tasks: (0..4)
                .map(|id| Task {
                    id,
                    resource_idx: None,
                })
                .collect(),
            score: None,
            score_director_toml,
        }
    }
}

/// Every assigned task costs its resource index plus one.
fn define_constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.resource_idx.is_none())
            .penalize(HardSoftScore::ONE_HARD)
            .named("unassigned task"),
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.resource_idx.is_some())
            .penalize(|task: &Task| {
                HardSoftScore::of(0, task.resource_idx.unwrap_or_default() as i64 + 1)
            })
            .named("placement cost"),
    )
}

/// Only assignment counts; placements are free.
fn relaxed_constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::tasks())
        .filter(|task: &Task| task.resource_idx.is_none())
        .penalize(HardSoftScore::ONE_HARD)
        .named("unassigned task (relaxed)"),)
}

pub const CONSTRUCTION_TOML: &str = r#"
random_seed = 7

[[phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"
"#;

fn director_config(solution: &Plan, _base: SolverConfig) -> SolverConfig {
    let toml = format!("{CONSTRUCTION_TOML}\n{}", solution.score_director_toml);
    SolverConfig::from_toml_str(&toml).expect("the fixture config parses")
}
