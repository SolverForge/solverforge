/* Task scheduling test fixtures.

Provides data types for a simple task scheduling problem.
Each task has a priority that can be assigned.

# Example

```
use solverforge_test::task::{Task, TaskSolution};

let solution = TaskSolution::new(vec![
Task::with_priority(1),
Task::with_priority(2),
Task::unassigned(),
]);
assert_eq!(solution.tasks.len(), 3);
```
*/

use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use std::any::TypeId;

// A task entity with an optional priority.
#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    pub priority: Option<i32>,
}

impl Task {
    pub fn new(priority: Option<i32>) -> Self {
        Self { priority }
    }

    pub fn with_priority(priority: i32) -> Self {
        Self {
            priority: Some(priority),
        }
    }

    pub fn unassigned() -> Self {
        Self { priority: None }
    }
}

// Task scheduling solution.
#[derive(Clone, Debug)]
pub struct TaskSolution {
    pub tasks: Vec<Task>,
    pub score: Option<SoftScore>,
}

impl TaskSolution {
    pub fn new(tasks: Vec<Task>) -> Self {
        Self { tasks, score: None }
    }

    pub fn unassigned(n: usize) -> Self {
        let tasks = (0..n).map(|_| Task::unassigned()).collect();
        Self { tasks, score: None }
    }
}

impl PlanningSolution for TaskSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

pub fn get_tasks(s: &TaskSolution) -> &Vec<Task> {
    &s.tasks
}

pub fn get_tasks_mut(s: &mut TaskSolution) -> &mut Vec<Task> {
    &mut s.tasks
}

pub fn get_priority(s: &TaskSolution, idx: usize) -> Option<i32> {
    s.tasks.get(idx).and_then(|t| t.priority)
}

pub fn set_priority(s: &mut TaskSolution, idx: usize, v: Option<i32>) {
    if let Some(task) = s.tasks.get_mut(idx) {
        task.priority = v;
    }
}

pub fn create_task_descriptor() -> SolutionDescriptor {
    let extractor = Box::new(EntityCollectionExtractor::new(
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
