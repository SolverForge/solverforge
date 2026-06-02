// Tests for list ruin move selector.

use crate::heuristic::r#move::{ListRuinMove, Move};
use crate::heuristic::selector::list_ruin::ListRuinMoveSelector;
use crate::heuristic::selector::move_selector::{MoveCandidateRef, MoveCursor};
use crate::heuristic::selector::MoveSelector;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use std::any::TypeId;

#[derive(Clone, Debug)]
struct Route {
    stops: Vec<i32>,
}

#[derive(Clone, Debug)]
struct VrpSolution {
    routes: Vec<Route>,
    score: Option<SoftScore>,
}

impl PlanningSolution for VrpSolution {
    type Score = SoftScore;
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
fn list_get(s: &VrpSolution, entity_idx: usize, pos: usize) -> Option<i32> {
    s.routes
        .get(entity_idx)
        .and_then(|r| r.stops.get(pos))
        .copied()
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
fn invalid_owner(_: &VrpSolution, element: &i32) -> Option<usize> {
    (*element == 99).then_some(99)
}
fn fixed_owner(_: &VrpSolution, element: &i32) -> Option<usize> {
    Some((*element / 100) as usize)
}
fn only_two_owner(_: &VrpSolution, element: &i32) -> Option<usize> {
    if *element == 2 {
        Some(0)
    } else {
        Some(99)
    }
}
fn precedence_element_count(_: &VrpSolution) -> usize {
    3
}
fn precedence_index_to_element(_: &VrpSolution, idx: usize) -> i32 {
    i32::try_from(idx + 1).expect("test element index fits i32")
}
fn precedence_successors(_: &VrpSolution, element: i32, out: &mut Vec<i32>) {
    if element == 1 {
        out.push(2);
    }
}

fn create_director(routes: Vec<Vec<i32>>) -> ScoreDirector<VrpSolution, ()> {
    let routes = routes.into_iter().map(|stops| Route { stops }).collect();
    let solution = VrpSolution {
        routes,
        score: None,
    };
    let descriptor = SolutionDescriptor::new("VrpSolution", TypeId::of::<VrpSolution>());
    ScoreDirector::simple(solution, descriptor, |s, _| s.routes.len())
}

#[test]
fn generates_list_ruin_moves() {
    let director = create_director(vec![vec![1, 2, 3, 4, 5]]);

    let selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        2,
        3,
        entity_count,
        list_len,
        list_get,
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
        list_get,
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
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    );

    let moves: Vec<_> = selector.iter_moves(&director).collect();
    assert!(moves.is_empty());
}

#[test]
fn empty_lists_should_not_reduce_moves_per_step() {
    let director = create_director(vec![vec![], vec![1, 2, 3]]);

    let selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        1,
        2,
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_moves_per_step(10)
    .with_seed(42);

    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(
        moves.len(),
        10,
        "empty routes should not consume moves_per_step attempts when non-empty routes exist"
    );

    // Some moves may be None due to empty list selection
    // All returned moves should be valid
    for m in &moves {
        assert_eq!(m.entity_index(), 1);
        assert!((1..=2).contains(&m.ruin_count()));
    }
}

#[test]
fn max_source_list_len_filters_long_routes() {
    let director = create_director(vec![vec![1, 2, 3], vec![4, 5, 6, 7, 8]]);

    let selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        1,
        2,
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_max_source_list_len(Some(3))
    .with_moves_per_step(10)
    .with_seed(42);

    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 10);
    assert!(
        moves.iter().all(|m| m.entity_index() == 0),
        "long routes should not be eligible as list-ruin sources"
    );
}

#[test]
fn skips_invalid_owner_elements_before_building_moves() {
    let director = create_director(vec![vec![99, 1]]);

    let selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        2,
        2,
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_element_owner_fn(Some(invalid_owner))
    .with_moves_per_step(3)
    .with_seed(42);

    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 3);
    for m in &moves {
        assert_eq!(m.element_indices(), &[1]);
        assert!(m.is_doable(&director));
    }
}

#[test]
fn owner_restricted_ruin_recreates_only_into_allowed_owner() {
    let mut director = create_director(vec![vec![100], vec![]]);

    let selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        1,
        1,
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_element_owner_fn(Some(fixed_owner))
    .with_moves_per_step(1)
    .with_seed(42);

    let mov = selector
        .iter_moves(&director)
        .next()
        .expect("owner-restricted ruin move");
    mov.do_move(&mut director);

    assert_eq!(
        director.working_solution().routes[0].stops,
        Vec::<i32>::new()
    );
    assert_eq!(director.working_solution().routes[1].stops, vec![100]);
}

#[test]
fn precedence_ruin_recreate_skips_cycle_forming_insertions() {
    let mut director = create_director(vec![vec![1, 2, 3]]);
    let mov = ListRuinMove::<VrpSolution, i32>::new(
        0,
        &[1],
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_precedence_hooks(
        Some(precedence_element_count),
        Some(precedence_index_to_element),
        Some(precedence_successors),
    );

    mov.do_move(&mut director);

    assert_eq!(director.working_solution().routes[0].stops, vec![1, 2, 3]);
}

#[test]
fn precedence_ruin_cursor_passes_recreate_filter_to_moves() {
    let mut director = create_director(vec![vec![1, 2, 3]]);
    let selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        1,
        1,
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_element_owner_fn(Some(only_two_owner))
    .with_precedence_hooks(
        Some(precedence_element_count),
        Some(precedence_index_to_element),
        Some(precedence_successors),
    )
    .with_moves_per_step(1)
    .with_seed(7);

    let mov = selector.iter_moves(&director).next().expect("ruin move");
    assert_eq!(mov.element_indices(), &[1]);
    mov.do_move(&mut director);

    assert_eq!(director.working_solution().routes[0].stops, vec![1, 2, 3]);
}

#[test]
fn seeded_selector_advances_between_steps() {
    let director = create_director(vec![vec![1, 2, 3, 4, 5], vec![6, 7, 8, 9, 10]]);

    let selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        1,
        3,
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_moves_per_step(6)
    .with_seed(42);

    let first: Vec<_> = selector
        .iter_moves(&director)
        .map(|m| (m.entity_index(), m.element_indices().to_vec()))
        .collect();
    let second: Vec<_> = selector
        .iter_moves(&director)
        .map(|m| (m.entity_index(), m.element_indices().to_vec()))
        .collect();

    let selector_again = ListRuinMoveSelector::<VrpSolution, i32>::new(
        1,
        3,
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_moves_per_step(6)
    .with_seed(42);
    let reproduced_first: Vec<_> = selector_again
        .iter_moves(&director)
        .map(|m| (m.entity_index(), m.element_indices().to_vec()))
        .collect();

    assert_eq!(first, reproduced_first);
    assert_ne!(
        first, second,
        "seeded list ruin selectors must advance their deterministic stream between iter_moves() calls"
    );
}

#[test]
fn size_returns_moves_per_step() {
    let director = create_director(vec![vec![1, 2, 3]]);

    let selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        1,
        2,
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_moves_per_step(7);

    assert_eq!(selector.size(&director), 7);
}

#[test]
fn cursor_generates_list_ruin_moves_lazily() {
    let director = create_director(vec![vec![1, 2, 3, 4, 5]]);

    let selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        1,
        2,
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_moves_per_step(4)
    .with_seed(7);

    let mut cursor = selector.open_cursor(&director);
    let first_id = cursor.next_candidate().expect("first candidate");
    let first = cursor.candidate(first_id).expect("first candidate ref");
    let MoveCandidateRef::Borrowed(first) = first else {
        panic!("list ruin cursor should yield borrowed candidates");
    };
    assert!((1..=2).contains(&first.ruin_count()));

    let second_id = cursor.next_candidate().expect("second candidate");
    let second = cursor.candidate(second_id).expect("second candidate ref");
    let MoveCandidateRef::Borrowed(second) = second else {
        panic!("list ruin cursor should yield borrowed candidates");
    };
    assert!((1..=2).contains(&second.ruin_count()));

    let remaining = std::iter::from_fn(|| cursor.next_candidate()).count();
    assert_eq!(remaining, 2);
    assert!(cursor.next_candidate().is_none());
}

#[test]
#[should_panic(expected = "min_ruin_count must be at least 1")]
fn panics_on_zero_min() {
    let _selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
        0,
        2,
        entity_count,
        list_len,
        list_get,
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
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    );
}
