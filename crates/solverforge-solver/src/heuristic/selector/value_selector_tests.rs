use super::*;
use solverforge_core::domain::{EntityCollectionExtractor, EntityDescriptor, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use std::any::TypeId;

#[derive(Clone, Debug)]
struct Task;

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
    let director = create_director(vec![Task]);
    let selector = StaticValueSelector::<TaskSolution, i32>::new(vec![1, 2, 3, 4, 5]);

    let values: Vec<_> = selector.iter(&director, 0, 0).collect();
    assert_eq!(values, vec![1, 2, 3, 4, 5]);
    assert_eq!(selector.size(&director, 0, 0), 5);
}
