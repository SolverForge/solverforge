//! Shared test utilities for solverforge-solver tests.
//!
//! This module provides common test infrastructure including:
//! - NQueensSolution: A classic N-Queens problem implementation for testing
//! - TestSolution: A minimal solution type for simple tests
//! - Helper functions for creating test directors and scopes

use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

use crate::scope::SolverScope;

// =============================================================================
// NQueensSolution - Full N-Queens implementation for testing
// =============================================================================

/// A queen in the N-Queens problem with a fixed column and variable row.
#[derive(Clone, Debug)]
pub struct Queen {
    pub column: i32,
    pub row: Option<i32>,
}

/// N-Queens problem solution for testing solver components.
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

pub fn get_queen_row(s: &NQueensSolution, idx: usize) -> Option<i32> {
    s.queens.get(idx).and_then(|q| q.row)
}

pub fn set_queen_row(s: &mut NQueensSolution, idx: usize, v: Option<i32>) {
    if let Some(queen) = s.queens.get_mut(idx) {
        queen.row = v;
    }
}

// Calculates the number of conflicts (row and diagonal) in an N-Queens solution.
// Returns a negative score where 0 means no conflicts.
pub fn calculate_conflicts(solution: &NQueensSolution) -> SimpleScore {
    let mut conflicts = 0i64;

    for (i, q1) in solution.queens.iter().enumerate() {
        if let Some(row1) = q1.row {
            for q2 in solution.queens.iter().skip(i + 1) {
                if let Some(row2) = q2.row {
                    // Row conflict
                    if row1 == row2 {
                        conflicts += 1;
                    }
                    // Diagonal conflict
                    let col_diff = (q2.column - q1.column).abs();
                    let row_diff = (row2 - row1).abs();
                    if col_diff == row_diff {
                        conflicts += 1;
                    }
                }
            }
        }
    }

    SimpleScore::of(-conflicts)
}

// Creates an NQueensSolution with queens at specified rows.
// The column is set to the index in the slice.
pub fn create_nqueens_solution(rows: &[i32]) -> NQueensSolution {
    let queens: Vec<_> = rows
        .iter()
        .enumerate()
        .map(|(col, &row)| Queen {
            column: col as i32,
            row: Some(row),
        })
        .collect();

    NQueensSolution {
        queens,
        score: None,
    }
}

// Creates an NQueensSolution with uninitialized queens.
pub fn create_uninitialized_nqueens(n: usize) -> NQueensSolution {
    let queens: Vec<_> = (0..n)
        .map(|col| Queen {
            column: col as i32,
            row: None,
        })
        .collect();

    NQueensSolution {
        queens,
        score: None,
    }
}

// Creates a score director for N-Queens with the conflict calculator.
pub fn create_nqueens_director(
    rows: &[i32],
) -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
    let solution = create_nqueens_solution(rows);
    create_nqueens_director_from_solution(solution)
}

// Creates a score director from an existing NQueensSolution.
pub fn create_nqueens_director_from_solution(
    solution: NQueensSolution,
) -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
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

    SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts)
}

// =============================================================================
// TestSolution - Minimal solution for simple tests
// =============================================================================

/// A minimal solution type for tests that don't need entity structure.
#[derive(Clone, Debug)]
pub struct TestSolution {
    pub score: Option<SimpleScore>,
}

impl PlanningSolution for TestSolution {
    type Score = SimpleScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

pub type TestDirector = SimpleScoreDirector<TestSolution, fn(&TestSolution) -> SimpleScore>;

fn calc_zero(_: &TestSolution) -> SimpleScore {
    SimpleScore::of(0)
}

// Creates a SolverScope with a TestSolution and no initial score.
pub fn create_test_scope() -> SolverScope<'static, TestSolution, TestDirector> {
    let desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>());
    let director = SimpleScoreDirector::with_calculator(
        TestSolution { score: None },
        desc,
        calc_zero as fn(&TestSolution) -> SimpleScore,
    );
    SolverScope::new(director)
}

// Creates a SolverScope with a TestSolution and a specific score.
pub fn create_test_scope_with_score(
    score: SimpleScore,
) -> SolverScope<
    'static,
    TestSolution,
    SimpleScoreDirector<TestSolution, impl Fn(&TestSolution) -> SimpleScore>,
> {
    let desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>());
    let score_clone = score;
    let director = SimpleScoreDirector::with_calculator(
        TestSolution { score: Some(score) },
        desc,
        move |_| score_clone,
    );
    let mut scope = SolverScope::new(director);
    scope.update_best_solution();
    scope
}
