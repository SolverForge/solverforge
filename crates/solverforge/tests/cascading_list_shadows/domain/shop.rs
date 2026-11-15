use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;
use solverforge::SolverConfig;

use super::{Machine, Task};

/// Machines whose tasks wait on tasks of other machines.
#[planning_solution(constraints = "define_constraints", config = "shop_config")]
#[shadow_variable_updates(
    list_owner = "machines",
    inverse_field = "machine",
    previous_field = "previous",
    next_field = "next",
    cascading_field = "start",
    cascading_compute = "compute_start",
    cascading_dependencies = "task_dependencies"
)]
pub struct Shop {
    /// The machines.
    #[planning_entity_collection]
    pub machines: Vec<Machine>,

    /// The tasks the machines process.
    #[problem_fact_collection]
    pub tasks: Vec<Task>,

    /// The shop's score.
    #[planning_score]
    pub score: Option<HardSoftScore>,
}

impl Shop {
    fn end(&self, task: usize) -> Option<i64> {
        let task = &self.tasks[task];
        task.start.map(|start| start + task.duration)
    }

    /// A task starts when the previous task on its machine and the task it
    /// waits on have both ended; unset while either is unset.
    pub fn compute_start(&self, task: usize) -> Option<i64> {
        let mut start = 0;
        if let Some(previous) = self.tasks[task].previous {
            start = start.max(self.end(previous)?);
        }
        if let Some(dependency) = self.tasks[task].waits_on {
            start = start.max(self.end(dependency)?);
        }
        Some(start)
    }

    /// The task `task` waits on, if any.
    pub fn task_dependencies(&self, task: usize) -> Option<usize> {
        self.tasks[task].waits_on
    }
}

/// Unassigned and unscheduled tasks are hard; every task's end is soft, so
/// any stale start changes the score.
pub fn define_constraints() -> impl ConstraintSet<Shop, HardSoftScore> {
    (
        ConstraintFactory::<Shop, HardSoftScore>::new()
            .for_each(Shop::tasks())
            .filter(|task: &Task| task.start.is_none())
            .penalize(HardSoftScore::ONE_HARD)
            .named("unscheduled task"),
        ConstraintFactory::<Shop, HardSoftScore>::new()
            .for_each(Shop::tasks())
            .filter(|task: &Task| task.start.is_some())
            .penalize(|task: &Task| HardSoftScore::of_soft(task.start.unwrap_or(0) + task.duration))
            .named("task end"),
    )
}

// Full assert checks every incremental score against a fresh calculation.
const SHOP_TOML: &str = r#"
environment_mode = "full_assert"
random_seed = 5

[[phases]]
type = "construction_heuristic"

[[phases]]
type = "local_search"

[phases.termination]
step_count_limit = 60
"#;

fn shop_config(_solution: &Shop, _base: SolverConfig) -> SolverConfig {
    SolverConfig::from_toml_str(SHOP_TOML).expect("shop fixture config must parse")
}
