//! Task scheduling test fixtures.
//!
//! Provides data types for a simple task scheduling problem.
//! Each task has a priority that can be assigned.
//!
//! # Example
//!
//! ```
//! use solverforge_test::task::{Task, TaskSolution};
//!
//! let solution = TaskSolution::new(vec![
//!     Task::with_priority(1),
//!     Task::with_priority(2),
//!     Task::unassigned(),
//! ]);
//! assert_eq!(solution.tasks.len(), 3);
//! ```

use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use std::any::TypeId;

/// A task entity with an optional priority.
#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    pub priority: Option<i32>,
}

impl Task {
    /// Creates a task with the given priority.
    pub fn new(priority: Option<i32>) -> Self {
        Self { priority }
    }

    /// Creates a task with an assigned priority.
    pub fn with_priority(priority: i32) -> Self {
        Self {
            priority: Some(priority),
        }
    }

    /// Creates a task with no priority assigned.
    pub fn unassigned() -> Self {
        Self { priority: None }
    }
}

/// Task scheduling solution.
#[derive(Clone, Debug)]
pub struct TaskSolution {
    pub tasks: Vec<Task>,
    pub score: Option<SimpleScore>,
}

impl TaskSolution {
    /// Creates a new task solution with the given tasks.
    pub fn new(tasks: Vec<Task>) -> Self {
        Self { tasks, score: None }
    }

    /// Creates a task solution with n unassigned tasks.
    pub fn unassigned(n: usize) -> Self {
        let tasks = (0..n).map(|_| Task::unassigned()).collect();
        Self { tasks, score: None }
    }
}

impl PlanningSolution for TaskSolution {
    type Score = SimpleScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

/// Gets a reference to the tasks vector.
pub fn get_tasks(s: &TaskSolution) -> &Vec<Task> {
    &s.tasks
}

/// Gets a mutable reference to the tasks vector.
pub fn get_tasks_mut(s: &mut TaskSolution) -> &mut Vec<Task> {
    &mut s.tasks
}

/// Gets the priority for a task at the given index.
pub fn get_priority(s: &TaskSolution, idx: usize) -> Option<i32> {
    s.tasks.get(idx).and_then(|t| t.priority)
}

/// Sets the priority for a task at the given index.
pub fn set_priority(s: &mut TaskSolution, idx: usize, v: Option<i32>) {
    if let Some(task) = s.tasks.get_mut(idx) {
        task.priority = v;
    }
}

/// Creates a SolutionDescriptor for TaskSolution.
pub fn create_task_descriptor() -> SolutionDescriptor {
    let extractor = Box::new(TypedEntityExtractor::new(
        "Task",
        "tasks",
        get_tasks,
        get_tasks_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);

    SolutionDescriptor::new("TaskSolution", TypeId::of::<TaskSolution>()).with_entity(entity_desc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let t1 = Task::new(Some(5));
        assert_eq!(t1.priority, Some(5));

        let t2 = Task::with_priority(10);
        assert_eq!(t2.priority, Some(10));

        let t3 = Task::unassigned();
        assert_eq!(t3.priority, None);
    }

    #[test]
    fn test_solution_creation() {
        let s1 = TaskSolution::unassigned(3);
        assert_eq!(s1.tasks.len(), 3);
        assert!(s1.tasks.iter().all(|t| t.priority.is_none()));

        let s2 = TaskSolution::new(vec![Task::with_priority(1), Task::with_priority(2)]);
        assert_eq!(s2.tasks.len(), 2);
    }

    #[test]
    fn test_get_set_priority() {
        let mut solution = TaskSolution::new(vec![Task::unassigned(), Task::unassigned()]);

        assert_eq!(get_priority(&solution, 0), None);
        set_priority(&mut solution, 0, Some(5));
        assert_eq!(get_priority(&solution, 0), Some(5));
    }
}
