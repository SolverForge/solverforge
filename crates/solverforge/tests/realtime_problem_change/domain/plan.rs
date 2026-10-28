use solverforge::prelude::*;
use solverforge::stream::joiner::equal;
use solverforge::stream::ConstraintFactory;
use solverforge::SolverConfig;

use super::{Resource, Task};

#[planning_solution(constraints = "define_constraints", config = "realtime_config")]
pub struct Plan {
    #[problem_fact_collection]
    pub resources: Vec<Resource>,

    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,

    pub phases: Phases,
}

/// Which configured phase sequence the job runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phases {
    /// Completes right after construction.
    ConstructionOnly,
    /// Explicit construction then unbounded local search.
    ConstructionAndSearch,
    /// No configured phases: the default runtime constructs and searches.
    Default,
}

// Tasks sharing a resource conflict, and each placement has its own cost, so
// local search keeps finding work until the job is cancelled.
fn define_constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.resource_idx.is_none())
            .penalize(HardSoftScore::ONE_HARD)
            .named("unassigned task"),
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .join(equal(|task: &Task| task.resource_idx))
            .penalize(HardSoftScore::ONE_SOFT)
            .named("shared resource"),
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.resource_idx.is_some())
            .penalize(|task: &Task| {
                let resource = task.resource_idx.unwrap_or_default() as i64;
                HardSoftScore::of(0, (task.weight * (resource + 3)) % 7)
            })
            .named("placement cost"),
    )
}

// No solver-level termination: a real-time job runs until it is cancelled.
// Full assert checks every incremental score against a fresh calculation.
const SEARCH_TOML: &str = r#"
environment_mode = "full_assert"
random_seed = 7

[[phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"

[[phases]]
type = "local_search"
"#;

const CONSTRUCTION_TOML: &str = r#"
environment_mode = "full_assert"
random_seed = 7

[[phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"
"#;

const DEFAULT_TOML: &str = r#"
environment_mode = "full_assert"
random_seed = 7
"#;

fn realtime_config(solution: &Plan, _base: SolverConfig) -> SolverConfig {
    let toml = match solution.phases {
        Phases::ConstructionOnly => CONSTRUCTION_TOML,
        Phases::ConstructionAndSearch => SEARCH_TOML,
        Phases::Default => DEFAULT_TOML,
    };
    SolverConfig::from_toml_str(toml).expect("real-time fixture config must parse")
}
