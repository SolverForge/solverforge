use super::*;
use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::ChangeMoveSelector;
use solverforge_core::domain::{EntityCollectionExtractor, EntityDescriptor, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use std::any::TypeId;

#[derive(Clone, Debug)]
struct Task {
    x: Option<i32>,
    y: Option<i32>,
}

#[derive(Clone, Debug)]
struct Sol {
    tasks: Vec<Task>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Sol {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_tasks(s: &Sol) -> &Vec<Task> {
    &s.tasks
}

fn get_tasks_mut(s: &mut Sol) -> &mut Vec<Task> {
    &mut s.tasks
}

fn get_x(s: &Sol, i: usize) -> Option<i32> {
    s.tasks.get(i).and_then(|t| t.x)
}

fn set_x(s: &mut Sol, i: usize, v: Option<i32>) {
    if let Some(t) = s.tasks.get_mut(i) {
        t.x = v;
    }
}

fn get_y(s: &Sol, i: usize) -> Option<i32> {
    s.tasks.get(i).and_then(|t| t.y)
}

fn set_y(s: &mut Sol, i: usize, v: Option<i32>) {
    if let Some(t) = s.tasks.get_mut(i) {
        t.y = v;
    }
}

fn create_director(tasks: Vec<Task>) -> ScoreDirector<Sol, ()> {
    let solution = Sol { tasks, score: None };
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Task",
        "tasks",
        get_tasks,
        get_tasks_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);
    let descriptor = SolutionDescriptor::new("Sol", TypeId::of::<Sol>()).with_entity(entity_desc);
    ScoreDirector::simple(solution, descriptor, |s, _| s.tasks.len())
}

#[test]
fn cartesian_product_arena_yields_all_pairs() {
    let director = create_director(vec![Task {
        x: Some(0),
        y: Some(0),
    }]);

    let x_selector = ChangeMoveSelector::simple(get_x, set_x, 0, "x", vec![1, 2]);
    let y_selector = ChangeMoveSelector::simple(get_y, set_y, 0, "y", vec![10, 20, 30]);

    let mut arena: CartesianProductArena<Sol, ChangeMove<Sol, i32>, ChangeMove<Sol, i32>> =
        CartesianProductArena::new();

    arena.populate_first(&x_selector, &director);
    arena.populate_second(&y_selector, &director);

    assert_eq!(arena.len(), 6);
    let pairs: Vec<_> = arena.iter_pairs().collect();
    assert_eq!(pairs.len(), 6);
}

#[test]
fn reset_clears_both_arenas() {
    let director = create_director(vec![Task {
        x: Some(0),
        y: Some(0),
    }]);

    let x_selector = ChangeMoveSelector::simple(get_x, set_x, 0, "x", vec![1, 2]);
    let y_selector = ChangeMoveSelector::simple(get_y, set_y, 0, "y", vec![10, 20]);

    let mut arena: CartesianProductArena<Sol, ChangeMove<Sol, i32>, ChangeMove<Sol, i32>> =
        CartesianProductArena::new();

    arena.populate_first(&x_selector, &director);
    arena.populate_second(&y_selector, &director);
    assert_eq!(arena.len(), 4);

    arena.reset();
    assert!(arena.is_empty());
    assert_eq!(arena.len(), 0);
}
