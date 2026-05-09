/* Test utilities for solverforge-solver

Provides common test fixtures used across the crate's test modules.
*/

use crate::scope::SolverScope;
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::director::score_director::ScoreDirector;
use std::any::TypeId;

#[derive(Clone, Debug, PartialEq)]
pub struct Queen {
    pub id: i64,
    pub column: i64,
    pub row: Option<i64>,
}

impl Queen {
    pub fn new(id: i64, column: i64, row: Option<i64>) -> Self {
        Self { id, column, row }
    }

    pub fn assigned(id: i64, column: i64, row: i64) -> Self {
        Self {
            id,
            column,
            row: Some(row),
        }
    }

    pub fn unassigned(id: i64, column: i64) -> Self {
        Self {
            id,
            column,
            row: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NQueensSolution {
    pub queens: Vec<Queen>,
    pub score: Option<SoftScore>,
}

impl NQueensSolution {
    pub fn new(queens: Vec<Queen>) -> Self {
        Self {
            queens,
            score: None,
        }
    }

    pub fn uninitialized(n: usize) -> Self {
        let queens = (0..n)
            .map(|i| Queen::unassigned(i as i64, i as i64))
            .collect();
        Self {
            queens,
            score: None,
        }
    }

    pub fn with_rows(rows: &[i64]) -> Self {
        let queens = rows
            .iter()
            .enumerate()
            .map(|(i, &row)| Queen::assigned(i as i64, i as i64, row))
            .collect();
        Self {
            queens,
            score: None,
        }
    }
}

impl PlanningSolution for NQueensSolution {
    type Score = SoftScore;

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

pub fn get_queen_row(s: &NQueensSolution, idx: usize, _variable_index: usize) -> Option<i64> {
    s.queens.get(idx).and_then(|q| q.row)
}

pub fn set_queen_row(s: &mut NQueensSolution, idx: usize, _variable_index: usize, v: Option<i64>) {
    if let Some(queen) = s.queens.get_mut(idx) {
        queen.row = v;
    }
}

pub fn calculate_conflicts(solution: &NQueensSolution) -> SoftScore {
    let mut conflicts = 0i64;
    let queens = &solution.queens;

    for i in 0..queens.len() {
        for j in (i + 1)..queens.len() {
            if let (Some(row_i), Some(row_j)) = (queens[i].row, queens[j].row) {
                if row_i == row_j {
                    conflicts += 1;
                }
                let col_diff = (queens[j].column - queens[i].column).abs();
                if (row_i - row_j).abs() == col_diff {
                    conflicts += 1;
                }
            }
        }
    }

    SoftScore::of(-conflicts)
}

pub fn create_nqueens_descriptor() -> SolutionDescriptor {
    let extractor = Box::new(EntityCollectionExtractor::new(
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

/* ============================================================================
TestSolution - a minimal solution type for solver tests
============================================================================
*/

/* A minimal test solution with just a score field.

This is useful for testing components like termination conditions
that only need to track score, not entities.
*/
#[derive(Clone, Debug)]
pub struct TestSolution {
    pub score: Option<SoftScore>,
}

impl TestSolution {
    pub fn new() -> Self {
        Self { score: None }
    }

    pub fn with_score(score: SoftScore) -> Self {
        Self { score: Some(score) }
    }
}

impl Default for TestSolution {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanningSolution for TestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

/// Type alias for a ScoreDirector with empty constraint set.
pub type TestDirector = ScoreDirector<TestSolution, ()>;

pub fn create_minimal_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
}

pub fn create_minimal_director() -> TestDirector {
    let solution = TestSolution::new();
    let descriptor = create_minimal_descriptor();
    ScoreDirector::simple(solution, descriptor, |_, _| 0)
}

/* ============================================================================
N-Queens director factories (solver-specific, using solverforge-scoring)
============================================================================
*/

pub fn create_nqueens_director(rows: &[i64]) -> ScoreDirector<NQueensSolution, ()> {
    let solution = NQueensSolution::with_rows(rows);
    let descriptor = create_nqueens_descriptor();
    ScoreDirector::simple(solution, descriptor, |s, _| s.queens.len())
}

pub fn create_simple_nqueens_director(n: usize) -> ScoreDirector<NQueensSolution, ()> {
    let solution = NQueensSolution::uninitialized(n);
    let descriptor = create_nqueens_descriptor();
    ScoreDirector::simple(solution, descriptor, |s, _| s.queens.len())
}

/* ============================================================================
SolverScope-specific helpers
============================================================================
*/

pub fn create_scope() -> SolverScope<'static, TestSolution, TestDirector> {
    let desc = create_minimal_descriptor();
    let director = ScoreDirector::simple(TestSolution::new(), desc, |_, _| 0);
    SolverScope::new(director)
}

/// Creates a SolverScope with a fixed score.
///
/// The score is set directly on the solution — no calculator is used.
pub fn create_scope_with_score(
    score: SoftScore,
) -> SolverScope<'static, TestSolution, TestDirector> {
    let desc = create_minimal_descriptor();
    let solution = TestSolution::with_score(score);
    let director = ScoreDirector::simple(solution.clone(), desc, |_, _| 0);
    let mut scope = SolverScope::new(director);
    scope.set_best_solution(solution, score);
    scope
}

#[cfg(test)]
#[path = "test_utils_tests.rs"]
mod tests;
