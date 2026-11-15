use solverforge::prelude::*;

use super::{Machine, Task};

// readme:begin
#[planning_solution]
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
    #[planning_entity_collection]
    pub machines: Vec<Machine>,

    #[problem_fact_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

impl Shop {
    fn end(&self, task: usize) -> Option<i64> {
        let task = &self.tasks[task];
        task.start.map(|start| start + task.duration)
    }

    // After the previous task on the machine and the task it waits on.
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

    pub fn task_dependencies(&self, task: usize) -> Option<usize> {
        self.tasks[task].waits_on
    }
}
// readme:end
