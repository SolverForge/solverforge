//! N-Queens Example
//!
//! The N-Queens problem is a classic constraint satisfaction problem where
//! N queens must be placed on an N×N chessboard such that no two queens
//! threaten each other.
//!
//! This example demonstrates how to model and solve the problem using SolverForge.

use rand::Rng;
use solverforge::prelude::*;
use solverforge::{planning_entity, planning_solution};

/// Planning Entity: A queen that needs to be placed.
#[planning_entity]
pub struct Queen {
    #[planning_id]
    pub id: i32,
    pub column: i32,
    #[planning_variable(allows_unassigned = true)]
    pub row: Option<i32>,
}

impl Queen {
    pub fn new(id: i32, column: i32) -> Self {
        Queen {
            id,
            column,
            row: None,
        }
    }
}

/// Planning Solution: The complete N-Queens problem.
#[planning_solution]
pub struct NQueensSolution {
    pub n: i32,
    #[planning_entity_collection]
    pub queens: Vec<Queen>,
    #[planning_score]
    pub score: Option<SimpleScore>,
}

impl NQueensSolution {
    /// Creates a new N-Queens problem of size n.
    pub fn new(n: i32) -> Self {
        let queens: Vec<Queen> = (0..n).map(|i| Queen::new(i, i)).collect();

        NQueensSolution {
            n,
            queens,
            score: None,
        }
    }

    /// Prints the board to stdout.
    pub fn print_board(&self) {
        println!("\n{}-Queens Solution (Score: {:?}):", self.n, self.score);
        println!("{}", "-".repeat((self.n as usize) * 2 + 1));

        for row_idx in 0..self.n {
            print!("|");
            for col_idx in 0..self.n {
                let queen_here = self
                    .queens
                    .iter()
                    .any(|q| q.column == col_idx && q.row == Some(row_idx));
                print!("{}", if queen_here { "Q|" } else { " |" });
            }
            println!();
        }
        println!("{}", "-".repeat((self.n as usize) * 2 + 1));
    }
}

/// Creates constraints for N-Queens using the fluent API.
///
/// Constraints:
/// 1. No two queens on the same row
/// 2. No two queens on the same ascending diagonal
/// 3. No two queens on the same descending diagonal
fn create_constraints() -> impl ConstraintSet<NQueensSolution, SimpleScore> {
    let factory = ConstraintFactory::<NQueensSolution, SimpleScore>::new();

    // Row conflict: two queens with same row
    let row_conflict = factory
        .clone()
        .for_each_unique_pair(
            |s: &NQueensSolution| s.queens.as_slice(),
            joiner::equal(|q: &Queen| q.row),
        )
        .filter(|a: &Queen, b: &Queen| a.row.is_some() && b.row.is_some())
        .penalize(SimpleScore::of(1))
        .as_constraint("Row conflict");

    // Ascending diagonal conflict: queens where (row - column) is the same
    let asc_diagonal = factory
        .clone()
        .for_each_unique_pair(
            |s: &NQueensSolution| s.queens.as_slice(),
            joiner::equal(|q: &Queen| q.row.map(|r| r - q.column)),
        )
        .filter(|a: &Queen, b: &Queen| a.row.is_some() && b.row.is_some())
        .penalize(SimpleScore::of(1))
        .as_constraint("Ascending diagonal conflict");

    // Descending diagonal conflict: queens where (row + column) is the same
    let desc_diagonal = factory
        .for_each_unique_pair(
            |s: &NQueensSolution| s.queens.as_slice(),
            joiner::equal(|q: &Queen| q.row.map(|r| r + q.column)),
        )
        .filter(|a: &Queen, b: &Queen| a.row.is_some() && b.row.is_some())
        .penalize(SimpleScore::of(1))
        .as_constraint("Descending diagonal conflict");

    (row_conflict, asc_diagonal, desc_diagonal)
}

/// Calculates the score directly (for validation and display).
fn calculate_score(solution: &NQueensSolution) -> SimpleScore {
    let mut conflicts = 0i64;

    for i in 0..solution.queens.len() {
        for j in (i + 1)..solution.queens.len() {
            let q1 = &solution.queens[i];
            let q2 = &solution.queens[j];

            // Skip if either queen is unassigned
            let (row1, row2) = match (q1.row, q2.row) {
                (Some(r1), Some(r2)) => (r1, r2),
                _ => continue,
            };

            // Same row conflict
            if row1 == row2 {
                conflicts += 1;
            }

            // Diagonal conflicts
            let col_diff = (q2.column - q1.column).abs();
            let row_diff = (row2 - row1).abs();
            if col_diff == row_diff {
                conflicts += 1;
            }
        }
    }

    SimpleScore::of(-conflicts)
}

/// Runs construction heuristic: round-robin row assignment.
fn construction_heuristic(
    director: &mut TypedScoreDirector<
        NQueensSolution,
        impl ConstraintSet<NQueensSolution, SimpleScore>,
    >,
    n: i32,
) -> SimpleScore {
    let _ = director.calculate_score();

    // Assign each queen to a unique row (round-robin)
    for queen_idx in 0..director.working_solution().queens.len() {
        if director.working_solution().queens[queen_idx].row.is_some() {
            continue;
        }

        let row = (queen_idx as i32) % n;
        director.before_variable_changed(queen_idx);
        director.working_solution_mut().queens[queen_idx].row = Some(row);
        director.after_variable_changed(queen_idx);
    }

    director.get_score()
}

/// Runs hill climbing local search.
fn hill_climbing(
    director: &mut TypedScoreDirector<
        NQueensSolution,
        impl ConstraintSet<NQueensSolution, SimpleScore>,
    >,
    n: i32,
    max_steps: u64,
) -> SimpleScore {
    let mut current_score = director.get_score();
    let mut rng = rand::thread_rng();
    let values: Vec<i32> = (0..n).collect();

    for _step in 0..max_steps {
        if current_score.is_feasible() {
            break; // Found optimal solution
        }

        // Generate random change move
        let queen_idx = rng.gen_range(0..director.working_solution().queens.len());
        let new_row = values[rng.gen_range(0..values.len())];
        let old_row = director.working_solution().queens[queen_idx].row;

        // Skip no-op
        if old_row == Some(new_row) {
            continue;
        }

        // Apply move
        director.before_variable_changed(queen_idx);
        director.working_solution_mut().queens[queen_idx].row = Some(new_row);
        director.after_variable_changed(queen_idx);
        let new_score = director.get_score();

        // Accept if better or equal
        if new_score >= current_score {
            current_score = new_score;
        } else {
            // Undo
            director.before_variable_changed(queen_idx);
            director.working_solution_mut().queens[queen_idx].row = old_row;
            director.after_variable_changed(queen_idx);
        }
    }

    current_score
}

fn main() {
    println!("SolverForge N-Queens Example");
    println!("============================\n");

    // Create a 4-Queens problem
    let n = 4;
    let solution = NQueensSolution::new(n);

    println!("Problem: {} queens on a {}x{} board", n, n, n);
    println!("Queens are fixed to columns, solver will assign rows.\n");

    // Create typed constraints and score director
    let constraints = create_constraints();
    let mut director = TypedScoreDirector::new(solution, constraints);

    // Phase 1: Construction heuristic
    println!("Running Construction Heuristic...");
    let score = construction_heuristic(&mut director, n);
    println!("After construction: {}", score);

    // Phase 2: Hill climbing local search
    println!("Running Hill Climbing (max 100 steps)...");
    let score = hill_climbing(&mut director, n, 100);
    println!("After local search: {}", score);

    // Display result
    let result = director.working_solution().clone();
    result.print_board();

    let score = result.score.unwrap_or_else(|| calculate_score(&result));
    if score.is_feasible() {
        println!("\nSolution is OPTIMAL! No queens threaten each other.");
    } else {
        println!(
            "\nSolution has {} conflicts (local optimum reached).",
            -score.score()
        );
    }

    // Show the queen positions
    println!("\nQueen positions:");
    for queen in &result.queens {
        println!(
            "  Queen {} at column {}, row {:?}",
            queen.id, queen.column, queen.row
        );
    }

    println!("\n--- Solving a larger problem ---\n");

    // Try 8-Queens
    let n = 8;
    let solution = NQueensSolution::new(n);
    let constraints = create_constraints();
    let mut director = TypedScoreDirector::new(solution, constraints);

    println!("Running Construction Heuristic...");
    let score = construction_heuristic(&mut director, n);
    println!("After construction: {}", score);

    println!("Running Hill Climbing (max 500 steps)...");
    let score = hill_climbing(&mut director, n, 500);
    println!("After local search: {}", score);

    let result = director.working_solution().clone();
    result.print_board();

    let score = result.score.unwrap_or_else(|| calculate_score(&result));
    if score.is_feasible() {
        println!("\nSolution is OPTIMAL!");
    } else {
        println!("\nReached local optimum with {} conflicts.", -score.score());
    }

    println!("\nSolverForge solver is working!");
}
