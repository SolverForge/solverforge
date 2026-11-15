use solverforge::prelude::*;

use super::{Machine, Task};

// Cascading shadows follow each element's list position, so the next field
// is required.
#[planning_solution]
#[shadow_variable_updates(
    list_owner = "machines",
    inverse_field = "machine",
    previous_field = "previous",
    cascading_field = "start",
    cascading_compute = "compute_start"
)]
pub struct Shop {
    #[planning_entity_collection]
    pub machines: Vec<Machine>,

    #[problem_fact_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

impl Shop {
    pub fn compute_start(&self, _task: usize) -> Option<i64> {
        Some(0)
    }
}
