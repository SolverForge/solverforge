/* N-Queens problem test fixtures.

Provides data types and pure functions for the N-Queens problem.
The N-Queens problem places N queens on an N×N chessboard such that
no two queens threaten each other.

# Example

```
use solverforge_test::nqueens::{NQueensSolution, calculate_conflicts};
use solverforge_core::score::SoftScore;

let solution = NQueensSolution::with_rows(&[1, 3, 0, 2]);
let score = calculate_conflicts(&solution);
assert_eq!(score, SoftScore::of(0)); // No conflicts
```
*/

use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use std::any::TypeId;

/* A queen entity in the N-Queens problem.

Each queen has:
- `id`: Unique identifier (typically the column index)
- `column`: The column position on the board (fixed/problem fact)
- `row`: The row position (planning variable, None if unassigned)
*/
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

/* N-Queens problem solution.

Contains a vector of queens and an optional score. The score is typically
calculated as the negative count of conflicts (row + diagonal).
*/
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

/// Sets the row value for a queen at the given index.
///
/// This is the typed setter for the planning variable.
pub fn set_queen_row(s: &mut NQueensSolution, idx: usize, _variable_index: usize, v: Option<i64>) {
    if let Some(queen) = s.queens.get_mut(idx) {
        queen.row = v;
    }
}

/// Calculates the number of conflicts in an N-Queens solution.
///
/// Counts row conflicts and diagonal conflicts between all pairs of queens.
pub fn calculate_conflicts(solution: &NQueensSolution) -> SoftScore {
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

/// Alias for `create_nqueens_descriptor` for backward compatibility.
pub fn create_test_descriptor() -> SolutionDescriptor {
    create_nqueens_descriptor()
}

/// Creates an N-Queens solution with optional rows (backward compatibility alias).
///
/// This matches the old `create_nqueens_solution` signature from scoring's test_utils.
pub fn create_nqueens_solution(rows: &[Option<i64>]) -> NQueensSolution {
    NQueensSolution::with_optional_rows(rows)
}

/// Alias for `get_queen_row`.
pub fn get_row(s: &NQueensSolution, idx: usize, variable_index: usize) -> Option<i64> {
    get_queen_row(s, idx, variable_index)
}

/// Alias for `set_queen_row`.
pub fn set_row(s: &mut NQueensSolution, idx: usize, variable_index: usize, v: Option<i64>) {
    set_queen_row(s, idx, variable_index, v)
}

#[cfg(test)]
#[path = "nqueens_tests.rs"]
mod tests;
