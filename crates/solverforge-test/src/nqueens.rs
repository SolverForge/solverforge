//! N-Queens problem test fixtures.
//!
//! Provides a complete N-Queens implementation for testing constraint satisfaction
//! and solver components. The N-Queens problem places N queens on an N×N chessboard
//! such that no two queens threaten each other.
//!
//! # Example
//!
//! ```ignore
//! use solverforge_test::nqueens::{create_nqueens_director, NQueensSolution};
//!
//! // Create a 4-queens problem with queens at rows [0, 2, 1, 3]
//! let director = create_nqueens_director(&[0, 2, 1, 3]);
//! let score = director.calculate_score();
//! ```

use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

/// A queen entity in the N-Queens problem.
///
/// Each queen has:
/// - `id`: Unique identifier (typically the column index)
/// - `column`: The column position on the board (fixed/problem fact)
/// - `row`: The row position (planning variable, None if unassigned)
#[derive(Clone, Debug, PartialEq)]
pub struct Queen {
    pub id: i64,
    pub column: i64,
    pub row: Option<i64>,
}

impl Queen {
    /// Creates a new queen at the given column with an optional row.
    pub fn new(id: i64, column: i64, row: Option<i64>) -> Self {
        Self { id, column, row }
    }

    /// Creates a queen with an assigned row.
    pub fn assigned(id: i64, column: i64, row: i64) -> Self {
        Self {
            id,
            column,
            row: Some(row),
        }
    }

    /// Creates a queen with no row assigned.
    pub fn unassigned(id: i64, column: i64) -> Self {
        Self {
            id,
            column,
            row: None,
        }
    }
}

/// N-Queens problem solution.
///
/// Contains a vector of queens and an optional score. The score is typically
/// calculated as the negative count of conflicts (row + diagonal).
#[derive(Clone, Debug)]
pub struct NQueensSolution {
    pub queens: Vec<Queen>,
    pub score: Option<SimpleScore>,
}

impl NQueensSolution {
    /// Creates a new N-Queens solution with the given queens.
    pub fn new(queens: Vec<Queen>) -> Self {
        Self {
            queens,
            score: None,
        }
    }

    /// Creates an N-Queens solution with n uninitialized queens.
    ///
    /// Queens are placed in columns 0..n with no row assigned.
    pub fn uninitialized(n: usize) -> Self {
        let queens = (0..n)
            .map(|i| Queen::unassigned(i as i64, i as i64))
            .collect();
        Self {
            queens,
            score: None,
        }
    }

    /// Creates an N-Queens solution with queens at the specified rows.
    ///
    /// Queens are placed in columns 0..n with rows from the provided slice.
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

    /// Creates an N-Queens solution with optional rows.
    pub fn with_optional_rows(rows: &[Option<i64>]) -> Self {
        let queens = rows
            .iter()
            .enumerate()
            .map(|(i, &row)| Queen::new(i as i64, i as i64, row))
            .collect();
        Self {
            queens,
            score: None,
        }
    }
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

/// Gets a reference to the queens vector.
pub fn get_queens(s: &NQueensSolution) -> &Vec<Queen> {
    &s.queens
}

/// Gets a mutable reference to the queens vector.
pub fn get_queens_mut(s: &mut NQueensSolution) -> &mut Vec<Queen> {
    &mut s.queens
}

/// Gets the row value for a queen at the given index.
///
/// This is the typed getter for the planning variable.
pub fn get_queen_row(s: &NQueensSolution, idx: usize) -> Option<i64> {
    s.queens.get(idx).and_then(|q| q.row)
}

/// Sets the row value for a queen at the given index.
///
/// This is the typed setter for the planning variable.
pub fn set_queen_row(s: &mut NQueensSolution, idx: usize, v: Option<i64>) {
    if let Some(queen) = s.queens.get_mut(idx) {
        queen.row = v;
    }
}

/// Calculates the number of conflicts in an N-Queens solution.
///
/// Counts row conflicts and diagonal conflicts between all pairs of queens.
/// Returns a negative score where 0 means no conflicts (optimal).
pub fn calculate_conflicts(solution: &NQueensSolution) -> SimpleScore {
    let mut conflicts = 0i64;
    let queens = &solution.queens;

    for i in 0..queens.len() {
        for j in (i + 1)..queens.len() {
            if let (Some(row_i), Some(row_j)) = (queens[i].row, queens[j].row) {
                // Row conflict: two queens on the same row
                if row_i == row_j {
                    conflicts += 1;
                }
                // Diagonal conflict: difference in rows equals difference in columns
                let col_diff = (queens[j].column - queens[i].column).abs();
                if (row_i - row_j).abs() == col_diff {
                    conflicts += 1;
                }
            }
        }
    }

    SimpleScore::of(-conflicts)
}

/// Creates a SolutionDescriptor for NQueensSolution.
pub fn create_nqueens_descriptor() -> SolutionDescriptor {
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

/// Creates a SimpleScoreDirector for N-Queens with queens at the specified rows.
///
/// This is the primary factory for creating test directors with initialized queens.
///
/// # Example
///
/// ```ignore
/// // Create a 4-queens problem with queens at rows [0, 2, 1, 3]
/// let director = create_nqueens_director(&[0, 2, 1, 3]);
/// ```
pub fn create_nqueens_director(
    rows: &[i64],
) -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
    let solution = NQueensSolution::with_rows(rows);
    let descriptor = create_nqueens_descriptor();
    SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts)
}

/// Creates a SimpleScoreDirector for N-Queens with n uninitialized queens.
///
/// This is useful for testing construction heuristics where queens start
/// without assigned rows.
///
/// # Example
///
/// ```ignore
/// // Create a 4-queens problem with no rows assigned
/// let director = create_simple_nqueens_director(4);
/// ```
pub fn create_simple_nqueens_director(
    n: usize,
) -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
    let solution = NQueensSolution::uninitialized(n);
    let descriptor = create_nqueens_descriptor();
    SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts)
}

/// Creates a SimpleScoreDirector for N-Queens with optional rows.
///
/// This is useful when you need a mix of initialized and uninitialized queens.
pub fn create_nqueens_director_optional(
    rows: &[Option<i64>],
) -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
    let solution = NQueensSolution::with_optional_rows(rows);
    let descriptor = create_nqueens_descriptor();
    SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_scoring::ScoreDirector;

    #[test]
    fn test_queen_creation() {
        let q1 = Queen::new(0, 0, Some(1));
        assert_eq!(q1.id, 0);
        assert_eq!(q1.column, 0);
        assert_eq!(q1.row, Some(1));

        let q2 = Queen::assigned(1, 1, 2);
        assert_eq!(q2.row, Some(2));

        let q3 = Queen::unassigned(2, 2);
        assert_eq!(q3.row, None);
    }

    #[test]
    fn test_solution_creation() {
        let s1 = NQueensSolution::uninitialized(4);
        assert_eq!(s1.queens.len(), 4);
        assert!(s1.queens.iter().all(|q| q.row.is_none()));

        let s2 = NQueensSolution::with_rows(&[0, 2, 1, 3]);
        assert_eq!(s2.queens.len(), 4);
        assert_eq!(s2.queens[0].row, Some(0));
        assert_eq!(s2.queens[1].row, Some(2));
    }

    #[test]
    fn test_conflict_calculation_no_conflicts() {
        // A valid 4-queens solution: rows [1, 3, 0, 2]
        let solution = NQueensSolution::with_rows(&[1, 3, 0, 2]);
        let score = calculate_conflicts(&solution);
        assert_eq!(score, SimpleScore::of(0));
    }

    #[test]
    fn test_conflict_calculation_row_conflict() {
        // Two queens on the same row
        let solution = NQueensSolution::with_rows(&[0, 0, 2, 3]);
        let score = calculate_conflicts(&solution);
        assert!(score < SimpleScore::of(0));
    }

    #[test]
    fn test_conflict_calculation_diagonal_conflict() {
        // Diagonal conflict: queens at (0,0) and (1,1)
        let solution = NQueensSolution::with_rows(&[0, 1, 3, 2]);
        let score = calculate_conflicts(&solution);
        assert!(score < SimpleScore::of(0));
    }

    #[test]
    fn test_create_nqueens_director() {
        let mut director = create_nqueens_director(&[0, 0, 0, 0]);
        let score = director.calculate_score();
        // All queens on row 0 = many conflicts
        assert!(score < SimpleScore::of(0));
    }

    #[test]
    fn test_create_simple_nqueens_director() {
        let director = create_simple_nqueens_director(4);
        assert_eq!(director.working_solution().queens.len(), 4);
        assert!(director
            .working_solution()
            .queens
            .iter()
            .all(|q| q.row.is_none()));
    }
}
