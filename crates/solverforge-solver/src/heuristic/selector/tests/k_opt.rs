//! Tests for k-opt move selector.

use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::entity::FromSolutionEntitySelector;
use crate::heuristic::selector::k_opt::{
    binomial, CutCombinationIterator, KOptConfig, KOptMoveSelector,
};
use crate::heuristic::selector::MoveSelector;

#[derive(Clone, Debug)]
struct Tour {
    cities: Vec<i32>,
}

#[derive(Clone, Debug)]
struct TspSolution {
    tours: Vec<Tour>,
    score: Option<SimpleScore>,
}

impl PlanningSolution for TspSolution {
    type Score = SimpleScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_tours(s: &TspSolution) -> &Vec<Tour> {
    &s.tours
}
fn get_tours_mut(s: &mut TspSolution) -> &mut Vec<Tour> {
    &mut s.tours
}
fn list_len(s: &TspSolution, entity_idx: usize) -> usize {
    s.tours.get(entity_idx).map_or(0, |t| t.cities.len())
}
fn sublist_remove(s: &mut TspSolution, entity_idx: usize, start: usize, end: usize) -> Vec<i32> {
    s.tours
        .get_mut(entity_idx)
        .map(|t| t.cities.drain(start..end).collect())
        .unwrap_or_default()
}
fn sublist_insert(s: &mut TspSolution, entity_idx: usize, pos: usize, items: Vec<i32>) {
    if let Some(t) = s.tours.get_mut(entity_idx) {
        for (i, item) in items.into_iter().enumerate() {
            t.cities.insert(pos + i, item);
        }
    }
}

fn create_director(
    tours: Vec<Tour>,
) -> SimpleScoreDirector<TspSolution, impl Fn(&TspSolution) -> SimpleScore> {
    let solution = TspSolution { tours, score: None };
    let extractor = Box::new(TypedEntityExtractor::new(
        "Tour",
        "tours",
        get_tours,
        get_tours_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Tour", TypeId::of::<Tour>(), "tours").with_extractor(extractor);
    let descriptor = SolutionDescriptor::new("TspSolution", TypeId::of::<TspSolution>())
        .with_entity(entity_desc);
    SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
}

#[test]
fn cut_combination_iterator_basic() {
    // For k=3, len=8, min_seg=1:
    // We need 4 segments of length >= 1
    // Cuts can be at positions 1-7 (not 0 or 8)
    // First combination: [1, 2, 3]
    let mut iter = CutCombinationIterator::new(3, 8, 1, 0);

    let first = iter.next().unwrap();
    assert_eq!(first.len(), 3);
    assert_eq!(first[0].position(), 1);
    assert_eq!(first[1].position(), 2);
    assert_eq!(first[2].position(), 3);

    // Count total combinations
    let count = 1 + iter.count(); // +1 for first we already took
                                  // C(8 - 4 + 3, 3) = C(7, 3) = 35
    assert_eq!(count, 35);
}

#[test]
fn cut_combination_too_short() {
    // Route too short for 3 cuts with min_seg=2
    // Need 4 segments * 2 = 8 elements minimum
    let mut iter = CutCombinationIterator::new(3, 6, 2, 0);
    assert!(iter.next().is_none());
}

#[test]
fn binomial_coefficient() {
    assert_eq!(binomial(5, 2), 10);
    assert_eq!(binomial(7, 3), 35);
    assert_eq!(binomial(10, 5), 252);
}

#[test]
fn selector_generates_moves() {
    let tours = vec![Tour {
        cities: vec![1, 2, 3, 4, 5, 6, 7, 8],
    }];
    let director = create_director(tours);

    let config = KOptConfig::new(3);

    let selector = KOptMoveSelector::<TspSolution, i32, _>::new(
        FromSolutionEntitySelector::new(0),
        config,
        list_len,
        sublist_remove,
        sublist_insert,
        "cities",
        0,
    );

    let moves: Vec<_> = selector.iter_moves(&director).collect();

    // 35 cut combinations × 7 patterns = 245 moves
    assert_eq!(moves.len(), 245);

    // All moves should be doable
    for m in &moves {
        assert!(m.is_doable(&director), "Move not doable: {:?}", m);
    }
}

#[test]
fn selector_size_matches_iteration() {
    let tours = vec![Tour {
        cities: vec![1, 2, 3, 4, 5, 6, 7, 8],
    }];
    let director = create_director(tours);

    let config = KOptConfig::new(3);

    let selector = KOptMoveSelector::<TspSolution, i32, _>::new(
        FromSolutionEntitySelector::new(0),
        config,
        list_len,
        sublist_remove,
        sublist_insert,
        "cities",
        0,
    );

    let size = selector.size(&director);
    let actual_count = selector.iter_moves(&director).count();

    assert_eq!(size, actual_count);
}
