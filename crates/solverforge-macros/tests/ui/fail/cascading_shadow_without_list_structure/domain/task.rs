use solverforge::prelude::*;

/// A task with a duration that may wait on a task of another machine.
#[problem_fact]
pub struct Task {
    /// Stable task id.
    #[planning_id]
    pub id: usize,
    /// Processing time.
    pub duration: i64,
    /// The task that must end before this one starts.
    pub waits_on: Option<usize>,
    /// The machine holding the task (list shadow).
    pub machine: Option<usize>,
    /// The task before this one on its machine (list shadow).
    pub previous: Option<usize>,
    /// The task after this one on its machine (list shadow).
    pub next: Option<usize>,
    /// When the task starts (cascading list shadow).
    pub start: Option<i64>,
}
