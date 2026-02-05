//! Shared test utilities for solverforge-scoring tests.
//!
//! This module provides common test infrastructure including:
//! - NQueensSolution: A classic N-Queens problem implementation for scoring tests
//! - Helper functions for creating descriptors and directors

use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use std::any::TypeId;

use crate::SimpleScoreDirector;

// =============================================================================
// NQueensSolution for scoring tests
// =============================================================================

/// A queen in the N-Queens problem with id and variable row.
#[derive(Clone, Debug, PartialEq)]
pub struct Queen {
    pub id: i64,
    pub row: Option<i32>,
}

/// N-Queens problem solution for testing scoring components.
#[derive(Clone, Debug)]
pub struct NQueensSolution {
    pub queens: Vec<Queen>,
    pub score: Option<SimpleScore>,
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

pub fn get_queens(s: &NQueensSolution) -> &Vec<Queen> {
    &s.queens
}

pub fn get_queens_mut(s: &mut NQueensSolution) -> &mut Vec<Queen> {
    &mut s.queens
}

// Typed getter - zero erasure
pub fn get_row(s: &NQueensSolution, idx: usize) -> Option<i32> {
    s.queens.get(idx).and_then(|q| q.row)
}

// Typed setter - zero erasure
pub fn set_row(s: &mut NQueensSolution, idx: usize, v: Option<i32>) {
    if let Some(q) = s.queens.get_mut(idx) {
        q.row = v;
    }
}

// Calculates the number of conflicts (row and diagonal) in an N-Queens solution.
// Returns a negative score where 0 means no conflicts.
pub fn calculate_conflicts(solution: &NQueensSolution) -> SimpleScore {
    let mut conflicts = 0i64;
    let queens = &solution.queens;

    for i in 0..queens.len() {
        for j in (i + 1)..queens.len() {
            if let (Some(row_i), Some(row_j)) = (queens[i].row, queens[j].row) {
                // Row conflict
                if row_i == row_j {
                    conflicts += 1;
                }
                // Diagonal conflict
                let col_diff = (j - i) as i32;
                if (row_i - row_j).abs() == col_diff {
                    conflicts += 1;
                }
            }
        }
    }

    SimpleScore::of(-conflicts)
}

// Creates a SolutionDescriptor for NQueensSolution.
pub fn create_test_descriptor() -> SolutionDescriptor {
    let extractor = Box::new(TypedEntityExtractor::new(
        "Queen",
        "queens",
        get_queens,
        get_queens_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Queen", TypeId::of::<Queen>(), "queens").with_extractor(extractor);

    SolutionDescriptor::new("NQueensSolution", TypeId::of::<NQueensSolution>())
        .with_entity(entity_desc)
}

// Creates an NQueensSolution with queens at specified rows.
pub fn create_nqueens_solution(rows: &[Option<i32>]) -> NQueensSolution {
    let queens: Vec<_> = rows
        .iter()
        .enumerate()
        .map(|(id, &row)| Queen { id: id as i64, row })
        .collect();

    NQueensSolution {
        queens,
        score: None,
    }
}

// Creates a SimpleScoreDirector for NQueensSolution.
pub fn create_nqueens_director(
    rows: &[Option<i32>],
) -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
    let solution = create_nqueens_solution(rows);
    let descriptor = create_test_descriptor();
    SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts)
}
