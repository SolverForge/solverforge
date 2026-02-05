//! Shared test infrastructure for decorator tests.
//!
//! Re-exports Task-based fixtures from solverforge-test.

pub use solverforge_test::task::{
    create_task_director as create_director, get_priority, set_priority, Task, TaskSolution,
};
