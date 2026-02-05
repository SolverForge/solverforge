//! Tests for entity placers.

use super::*;
use crate::heuristic::selector::{FromSolutionEntitySelector, StaticTypedValueSelector};
use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

#[derive(Clone, Debug)]
struct Queen {
    row: Option<i32>,
}

#[derive(Clone, Debug)]
struct NQueensSolution {
    queens: Vec<Queen>,
    score: Option<SimpleScore>,
}

impl PlanningSolution for NQueensSolution {
    type Score = SimpleScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_queens(s: &NQueensSolution) -> &Vec<Queen> {
    &s.queens
}

fn get_queens_mut(s: &mut NQueensSolution) -> &mut Vec<Queen> {
    &mut s.queens
}

// Typed getter - zero erasure
fn get_queen_row(s: &NQueensSolution, idx: usize) -> Option<i32> {
    s.queens.get(idx).and_then(|q| q.row)
}

// Typed setter - zero erasure
fn set_queen_row(s: &mut NQueensSolution, idx: usize, v: Option<i32>) {
    if let Some(queen) = s.queens.get_mut(idx) {
        queen.row = v;
    }
}

fn create_test_director(
    initialized: &[bool],
) -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
    let queens: Vec<_> = initialized
        .iter()
        .enumerate()
        .map(|(i, init)| Queen {
            row: if *init { Some(i as i32) } else { None },
        })
        .collect();

    let solution = NQueensSolution {
        queens,
        score: None,
    };

    let extractor = Box::new(TypedEntityExtractor::new(
        "Queen",
        "queens",
        get_queens,
        get_queens_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Queen", TypeId::of::<Queen>(), "queens").with_extractor(extractor);

    let descriptor = SolutionDescriptor::new("NQueensSolution", TypeId::of::<NQueensSolution>())
        .with_entity(entity_desc);

    SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
}

#[test]
fn test_queued_placer_all_uninitialized() {
    let director = create_test_director(&[false, false, false]);

    let entity_selector = FromSolutionEntitySelector::new(0);
    let value_selector = StaticTypedValueSelector::new(vec![0i32, 1, 2]);

    let placer = QueuedEntityPlacer::new(
        entity_selector,
        value_selector,
        get_queen_row,
        set_queen_row,
        0,
        "row",
    );

    let placements = placer.get_placements(&director);

    // All 3 entities should have placements
    assert_eq!(placements.len(), 3);

    // Each should have 3 moves (one per value)
    for p in &placements {
        assert_eq!(p.moves.len(), 3);
    }
}

#[test]
fn test_queued_placer_some_initialized() {
    // First and third are initialized, middle is not
    let director = create_test_director(&[true, false, true]);

    let entity_selector = FromSolutionEntitySelector::new(0);
    let value_selector = StaticTypedValueSelector::new(vec![0i32, 1, 2]);

    let placer = QueuedEntityPlacer::new(
        entity_selector,
        value_selector,
        get_queen_row,
        set_queen_row,
        0,
        "row",
    );

    let placements = placer.get_placements(&director);

    // Only 1 entity (index 1) should have a placement
    assert_eq!(placements.len(), 1);
    assert_eq!(placements[0].entity_ref.entity_index, 1);
}

#[test]
fn test_queued_placer_all_initialized() {
    let director = create_test_director(&[true, true, true]);

    let entity_selector = FromSolutionEntitySelector::new(0);
    let value_selector = StaticTypedValueSelector::new(vec![0i32, 1, 2]);

    let placer = QueuedEntityPlacer::new(
        entity_selector,
        value_selector,
        get_queen_row,
        set_queen_row,
        0,
        "row",
    );

    let placements = placer.get_placements(&director);

    // No placements - all already initialized
    assert_eq!(placements.len(), 0);
}

#[test]
fn test_sorted_entity_placer_descending() {
    // Create 3 uninitialized queens
    let director = create_test_director(&[false, false, false]);

    let entity_selector = FromSolutionEntitySelector::new(0);
    let value_selector = StaticTypedValueSelector::new(vec![0i32, 1, 2]);

    let inner = QueuedEntityPlacer::new(
        entity_selector,
        value_selector,
        get_queen_row,
        set_queen_row,
        0,
        "row",
    );

    // Sort by entity index descending (2, 1, 0)
    fn descending_index(_s: &NQueensSolution, a: usize, b: usize) -> std::cmp::Ordering {
        b.cmp(&a)
    }

    let sorted = SortedEntityPlacer::new(inner, descending_index);
    let placements = sorted.get_placements(&director);

    assert_eq!(placements.len(), 3);
    assert_eq!(placements[0].entity_ref.entity_index, 2);
    assert_eq!(placements[1].entity_ref.entity_index, 1);
    assert_eq!(placements[2].entity_ref.entity_index, 0);
}
