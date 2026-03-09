//! Shared test infrastructure for decorator tests.
//!
//! Re-exports Task-based fixtures from solverforge-test and provides
//! director factory functions.

use solverforge_scoring::ScoreDirector;

pub use solverforge_test::task::{
    create_task_descriptor, get_priority, set_priority, Task, TaskSolution,
};

/// Creates a ScoreDirector for TaskSolution.
///
/// The score calculator returns zero (tasks have no inherent conflicts).
pub fn create_director(tasks: Vec<Task>) -> ScoreDirector<TaskSolution, ()> {
    let solution = TaskSolution::new(tasks);
    let descriptor = create_task_descriptor();
    ScoreDirector::simple(solution, descriptor, |s, _| s.tasks.len())
}
