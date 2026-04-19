use std::{
    any::TypeId,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use solverforge_core::{
    domain::{EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor},
    score::SoftScore,
};
use solverforge_scoring::ScoreDirector;

use super::super::test_utils::{create_director, get_priority, set_priority, Task};
use super::*;
use crate::heuristic::selector::move_selector::MoveSelector;
use crate::heuristic::selector::ChangeMoveSelector;

#[test]
fn limits_move_count() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        "priority",
        vec![10, 20, 30, 40, 50],
    );
    let limited = SelectedCountLimitMoveSelector::new(inner, 3);

    let moves: Vec<_> = limited.iter_moves(&director).collect();
    assert_eq!(moves.len(), 3);
    assert_eq!(limited.size(&director), 3);
}

#[test]
fn returns_all_when_under_limit() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner = ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![10, 20]);
    let limited = SelectedCountLimitMoveSelector::new(inner, 10);

    let moves: Vec<_> = limited.iter_moves(&director).collect();
    assert_eq!(moves.len(), 2);
    assert_eq!(limited.size(&director), 2);
}

#[test]
fn zero_limit_yields_nothing() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner =
        ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![10, 20, 30]);
    let limited = SelectedCountLimitMoveSelector::new(inner, 0);

    let moves: Vec<_> = limited.iter_moves(&director).collect();
    assert!(moves.is_empty());
    assert_eq!(limited.size(&director), 0);
}

// Test for: "the wrapper should limit inner child generation?"
#[test]
fn limit_caps_change_selector_value_generation() {
    let director = create_counted_director(vec![CountedTask { value: None }]);
    let limit = 3;
    let total = 10;
    let cloned = Arc::new(AtomicUsize::new(0));
    let values = (0..total)
        .map(|id| CountedValue {
            id,
            cloned: Arc::clone(&cloned),
        })
        .collect();

    let inner =
        ChangeMoveSelector::simple(get_counted_value, set_counted_value, 0, "counted", values);
    let limited = SelectedCountLimitMoveSelector::new(inner, limit);

    let moves: Vec<_> = limited.iter_moves(&director).collect();
    assert_eq!(moves.len(), limit);

    // if SelectedCountLimitMoveSelector properly limits the inner selector, then only `limit` values should have been cloned.
    // this is based on the assumption that the selector should limit the child generation of moves.
    assert_eq!(cloned.load(Ordering::SeqCst), limit);
}

#[derive(Clone, Debug)]
struct CountedTask {
    value: Option<CountedValue>,
}

#[derive(Clone, Debug)]
struct CountedSolution {
    tasks: Vec<CountedTask>,
    score: Option<SoftScore>,
}

impl PlanningSolution for CountedSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Debug)]
struct CountedValue {
    id: usize,
    cloned: Arc<AtomicUsize>,
}

impl Clone for CountedValue {
    fn clone(&self) -> Self {
        self.cloned.fetch_add(1, Ordering::SeqCst);
        Self {
            id: self.id,
            cloned: Arc::clone(&self.cloned),
        }
    }
}

impl PartialEq for CountedValue {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

fn get_counted_tasks(solution: &CountedSolution) -> &Vec<CountedTask> {
    &solution.tasks
}

fn get_counted_tasks_mut(solution: &mut CountedSolution) -> &mut Vec<CountedTask> {
    &mut solution.tasks
}

fn get_counted_value(solution: &CountedSolution, idx: usize) -> Option<CountedValue> {
    solution.tasks.get(idx).and_then(|task| task.value.clone())
}

fn set_counted_value(solution: &mut CountedSolution, idx: usize, value: Option<CountedValue>) {
    if let Some(task) = solution.tasks.get_mut(idx) {
        task.value = value;
    }
}

fn create_counted_director(tasks: Vec<CountedTask>) -> ScoreDirector<CountedSolution, ()> {
    let solution = CountedSolution { tasks, score: None };
    let extractor = Box::new(EntityCollectionExtractor::new(
        "CountedTask",
        "tasks",
        get_counted_tasks,
        get_counted_tasks_mut,
    ));
    let entity_desc = EntityDescriptor::new("CountedTask", TypeId::of::<CountedTask>(), "tasks")
        .with_extractor(extractor);
    let descriptor = SolutionDescriptor::new("CountedSolution", TypeId::of::<CountedSolution>())
        .with_entity(entity_desc);
    ScoreDirector::simple(solution, descriptor, |s, _| s.tasks.len())
}
