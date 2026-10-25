use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;
use solverforge::{MoveThreadCount, SolverConfig};

use super::{Resource, Task};

#[planning_solution(constraints = "define_constraints", config = "parallel_config")]
pub struct Plan {
    #[problem_fact_collection]
    pub resources: Vec<Resource>,

    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,

    pub move_threads: usize,
}

fn define_constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.resource_idx.is_none())
            .penalize(HardSoftScore::of(1, 0))
            .named("unassigned task"),
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.resource_idx.is_some())
            .penalize(|task: &Task| {
                let resource = task.resource_idx.unwrap_or_default() as i64;
                HardSoftScore::of(0, (task.weight * (resource + 3)) % 17)
            })
            .named("resource cost"),
    )
}

const PARALLEL_SOLVER_TOML: &str = r#"
random_seed = 7

[candidate_trace]
max_entries = 1_000_000

[[phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"

[[phases]]
type = "local_search"

[phases.termination]
step_count_limit = 150
"#;

fn parallel_config(solution: &Plan, _base: SolverConfig) -> SolverConfig {
    let mut config = SolverConfig::from_toml_str(PARALLEL_SOLVER_TOML)
        .expect("parallel local-search fixture config must parse");
    config.move_thread_count = if solution.move_threads == 0 {
        MoveThreadCount::Auto
    } else {
        MoveThreadCount::Count(solution.move_threads)
    };
    config
}
