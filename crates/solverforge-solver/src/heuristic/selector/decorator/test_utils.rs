//! Shared test infrastructure for decorator tests.

use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

#[derive(Clone, Debug)]
pub struct Task {
    pub priority: Option<i32>,
}

#[derive(Clone, Debug)]
pub struct TaskSolution {
    pub tasks: Vec<Task>,
    pub score: Option<SimpleScore>,
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

pub fn get_tasks(s: &TaskSolution) -> &Vec<Task> {
    &s.tasks
}

pub fn get_tasks_mut(s: &mut TaskSolution) -> &mut Vec<Task> {
    &mut s.tasks
}

pub fn get_priority(s: &TaskSolution, i: usize) -> Option<i32> {
    s.tasks.get(i).and_then(|t| t.priority)
}

pub fn set_priority(s: &mut TaskSolution, i: usize, v: Option<i32>) {
    if let Some(t) = s.tasks.get_mut(i) {
        t.priority = v;
    }
}

pub fn create_director(
    tasks: Vec<Task>,
) -> SimpleScoreDirector<TaskSolution, impl Fn(&TaskSolution) -> SimpleScore> {
    let solution = TaskSolution { tasks, score: None };
    let extractor = Box::new(TypedEntityExtractor::new(
        "Task",
        "tasks",
        get_tasks,
        get_tasks_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);
    let descriptor = SolutionDescriptor::new("TaskSolution", TypeId::of::<TaskSolution>())
        .with_entity(entity_desc);
    SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
}
