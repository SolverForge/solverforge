//! Tests for list ruin move selector.

use crate::heuristic::selector::list_ruin::ListRuinMoveSelector;
use crate::heuristic::selector::MoveSelector;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

#[derive(Clone, Debug)]
struct Route {
    stops: Vec<i32>,
}

#[derive(Clone, Debug)]
struct VrpSolution {
    routes: Vec<Route>,
    score: Option<SimpleScore>,
}

impl PlanningSolution for VrpSolution {
    type Score = SimpleScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn entity_count(s: &VrpSolution) -> usize {
    s.routes.len()
}
fn list_len(s: &VrpSolution, entity_idx: usize) -> usize {
    s.routes.get(entity_idx).map_or(0, |r| r.stops.len())
}
fn list_remove(s: &mut VrpSolution, entity_idx: usize, idx: usize) -> i32 {
    s.routes
        .get_mut(entity_idx)
        .map(|r| r.stops.remove(idx))
        .unwrap_or(0)
}
fn list_insert(s: &mut VrpSolution, entity_idx: usize, idx: usize, v: i32) {
    if let Some(r) = s.routes.get_mut(entity_idx) {
        r.stops.insert(idx, v);
    }
}

fn create_director(
    routes: Vec<Vec<i32>>,
) -> SimpleScoreDirector<VrpSolution, impl Fn(&VrpSolution) -> SimpleScore> {
    let routes = routes.into_iter().map(|stops| Route { stops }).collect();
    let solution = VrpSolution {
        routes,
        score: None,
    };
    let descriptor = SolutionDescriptor::new("VrpSolution", TypeId::of::<VrpSolution>());
    SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
}

#[test]
fn generates_list_ruin_moves() {
    let director = create_director(vec![vec![1, 2, 3, 4, 5]]);

    let selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        2,
        3,
        entity_count,
        list_len,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_moves_per_step(5);

    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 5);
    for m in &moves {
        let count = m.ruin_count();
        assert!((2..=3).contains(&count));
    }
}

#[test]
fn clamps_to_available_elements() {
    let director = create_director(vec![vec![1, 2]]);

    // Request more elements than available
    let selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        5,
        10,
        entity_count,
        list_len,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_moves_per_step(3);

    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 3);
    for m in &moves {
        assert!(m.ruin_count() <= 2);
    }
}

#[test]
fn empty_solution_yields_no_moves() {
    let director = create_director(vec![]);

    let selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        1,
        2,
        entity_count,
        list_len,
        list_remove,
        list_insert,
        "stops",
        0,
    );

    let moves: Vec<_> = selector.iter_moves(&director).collect();
    assert!(moves.is_empty());
}

#[test]
fn empty_list_yields_no_moves_for_that_entity() {
    let director = create_director(vec![vec![], vec![1, 2, 3]]);

    let selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        1,
        2,
        entity_count,
        list_len,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_moves_per_step(10)
    .with_seed(42);

    let moves: Vec<_> = selector.iter_moves(&director).collect();

    // Some moves may be None due to empty list selection
    // All returned moves should be valid
    for m in &moves {
        assert!(m.ruin_count() >= 1);
    }
}

#[test]
fn size_returns_moves_per_step() {
    let director = create_director(vec![vec![1, 2, 3]]);

    let selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        1,
        2,
        entity_count,
        list_len,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_moves_per_step(7);

    assert_eq!(selector.size(&director), 7);
}

#[test]
#[should_panic(expected = "min_ruin_count must be at least 1")]
fn panics_on_zero_min() {
    let _selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        0,
        2,
        entity_count,
        list_len,
        list_remove,
        list_insert,
        "stops",
        0,
    );
}

#[test]
#[should_panic(expected = "max_ruin_count must be >= min_ruin_count")]
fn panics_on_invalid_range() {
    let _selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        5,
        2,
        entity_count,
        list_len,
        list_remove,
        list_insert,
        "stops",
        0,
    );
}
