//! Tests for construction foragers.

use super::*;
use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::EntityReference;
use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

#[derive(Clone, Debug)]
struct Queen {
    row: Option<i64>,
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

fn get_queen_row(s: &NQueensSolution, idx: usize) -> Option<i64> {
    s.queens.get(idx).and_then(|q| q.row)
}

fn set_queen_row(s: &mut NQueensSolution, idx: usize, v: Option<i64>) {
    if let Some(queen) = s.queens.get_mut(idx) {
        queen.row = v;
    }
}

fn create_test_director(
) -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
    let solution = NQueensSolution {
        queens: vec![Queen { row: None }],
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

    SimpleScoreDirector::with_calculator(solution, descriptor, |sol| {
        let sum: i64 = sol.queens.iter().filter_map(|q| q.row).sum();
        SimpleScore::of(sum)
    })
}

type TestMove = ChangeMove<NQueensSolution, i64>;

fn create_placement() -> Placement<NQueensSolution, TestMove> {
    let entity_ref = EntityReference::new(0, 0);
    let moves: Vec<TestMove> = vec![
        ChangeMove::new(0, Some(1i64), get_queen_row, set_queen_row, "row", 0),
        ChangeMove::new(0, Some(5i64), get_queen_row, set_queen_row, "row", 0),
        ChangeMove::new(0, Some(3i64), get_queen_row, set_queen_row, "row", 0),
    ];
    Placement::new(entity_ref, moves)
}

#[test]
fn test_first_fit_forager() {
    let mut director = create_test_director();
    let mut placement = create_placement();

    let forager = FirstFitForager::<NQueensSolution, TestMove>::new();
    let selected_idx = forager.pick_move_index(&placement, &mut director);

    // First Fit should pick the first move (index 0)
    assert_eq!(selected_idx, Some(0));

    // Take move and execute
    if let Some(idx) = selected_idx {
        let m = placement.moves.swap_remove(idx);
        m.do_move(&mut director);
    }
}

#[test]
fn test_best_fit_forager() {
    let mut director = create_test_director();
    let mut placement = create_placement();

    let forager = BestFitForager::<NQueensSolution, TestMove>::new();
    let selected_idx = forager.pick_move_index(&placement, &mut director);

    // Best Fit should pick the move with highest score (index 1, value 5)
    assert_eq!(selected_idx, Some(1));

    // Take move and execute
    if let Some(idx) = selected_idx {
        let m = placement.moves.swap_remove(idx);
        m.do_move(&mut director);
        let score = director.calculate_score();
        assert_eq!(score, SimpleScore::of(5));
    }
}

#[test]
fn test_empty_placement() {
    let mut director = create_test_director();
    let placement: Placement<NQueensSolution, TestMove> =
        Placement::new(EntityReference::new(0, 0), vec![]);

    let forager = FirstFitForager::<NQueensSolution, TestMove>::new();
    let selected_idx = forager.pick_move_index(&placement, &mut director);

    assert!(selected_idx.is_none());
}

fn value_strength(m: &TestMove) -> i64 {
    m.to_value().copied().unwrap_or(0)
}

#[test]
fn test_weakest_fit_forager() {
    let mut director = create_test_director();
    let mut placement = create_placement(); // values: 1, 5, 3

    let forager = WeakestFitForager::<NQueensSolution, TestMove>::new(value_strength);
    let selected_idx = forager.pick_move_index(&placement, &mut director);

    // Weakest Fit should pick the move with lowest strength (index 0, value 1)
    assert_eq!(selected_idx, Some(0));

    if let Some(idx) = selected_idx {
        let m = placement.moves.swap_remove(idx);
        m.do_move(&mut director);
        let score = director.calculate_score();
        assert_eq!(score, SimpleScore::of(1));
    }
}

#[test]
fn test_strongest_fit_forager() {
    let mut director = create_test_director();
    let mut placement = create_placement(); // values: 1, 5, 3

    let forager = StrongestFitForager::<NQueensSolution, TestMove>::new(value_strength);
    let selected_idx = forager.pick_move_index(&placement, &mut director);

    // Strongest Fit should pick the move with highest strength (index 1, value 5)
    assert_eq!(selected_idx, Some(1));

    if let Some(idx) = selected_idx {
        let m = placement.moves.swap_remove(idx);
        m.do_move(&mut director);
        let score = director.calculate_score();
        assert_eq!(score, SimpleScore::of(5));
    }
}
