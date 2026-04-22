use super::*;
use solverforge_core::domain::{EntityCollectionExtractor, EntityDescriptor, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use std::any::TypeId;

#[derive(Clone, Debug)]
struct Task {
    id: usize,
    priority: Option<i32>,
}

#[derive(Clone, Debug)]
struct TaskSolution {
    tasks: Vec<Task>,
    score: Option<SoftScore>,
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

fn create_director(tasks: Vec<Task>) -> ScoreDirector<TaskSolution, ()> {
    let solution = TaskSolution { tasks, score: None };
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Task",
        "tasks",
        |s: &TaskSolution| &s.tasks,
        |s: &mut TaskSolution| &mut s.tasks,
    ));
    let entity_desc =
        EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);
    let descriptor = SolutionDescriptor::new("TaskSolution", TypeId::of::<TaskSolution>())
        .with_entity(entity_desc);
    ScoreDirector::simple(solution, descriptor, |s, _| s.tasks.len())
}

#[test]
fn test_static_value_selector_selector() {
    let director = create_director(vec![Task {
        id: 0,
        priority: None,
    }]);
    let selector = StaticValueSelector::<TaskSolution, i32>::new(vec![1, 2, 3, 4, 5]);

    let values: Vec<_> = selector.iter(&director, 0, 0).collect();
    assert_eq!(values, vec![1, 2, 3, 4, 5]);
    assert_eq!(selector.size(&director, 0, 0), 5);
}

#[test]
fn test_from_solution_value_selector_selector() {
    let director = create_director(vec![
        Task {
            id: 0,
            priority: Some(10),
        },
        Task {
            id: 1,
            priority: Some(20),
        },
    ]);

    let solution = director.working_solution();
    assert_eq!(solution.tasks[0].id, 0);
    assert_eq!(solution.tasks[1].id, 1);

    fn extract_priorities(s: &TaskSolution) -> Vec<i32> {
        s.tasks.iter().filter_map(|t| t.priority).collect()
    }

    let selector = FromSolutionValueSelector::new(extract_priorities);

    let values: Vec<_> = selector.iter(&director, 0, 0).collect();
    assert_eq!(values, vec![10, 20]);
}
