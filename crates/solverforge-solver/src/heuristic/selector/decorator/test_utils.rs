//! Shared test infrastructure for decorator tests.
//!
//! Re-exports Task-based fixtures from solverforge-test and provides
//! director factory functions.

use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;

pub use solverforge_test::task::{
    create_task_descriptor, get_priority, set_priority, Task, TaskSolution,
};

/// Creates a SimpleScoreDirector for TaskSolution.
///
/// The score calculator returns zero (tasks have no inherent conflicts).
pub fn create_director(
    tasks: Vec<Task>,
) -> SimpleScoreDirector<TaskSolution, impl Fn(&TaskSolution) -> SimpleScore> {
    let solution = TaskSolution::new(tasks);
    let descriptor = create_task_descriptor();
    SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
}
