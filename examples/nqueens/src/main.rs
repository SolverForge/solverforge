//! N-Queens Example
//!
//! The N-Queens problem is a classic constraint satisfaction problem where
//! N queens must be placed on an N×N chessboard such that no two queens
//! threaten each other.
//!
//! This example demonstrates how to model and solve the problem using SolverForge.

use std::any::TypeId;

use solverforge::prelude::*;
use solverforge::{
    ChangeMove, ChangeMoveSelector, EntityDescriptor,
    StepCountTermination, TypedEntityExtractor,
    ConstructionPhaseFactory, LocalSearchPhaseFactory, SolverPhaseFactory,
    QueuedEntityPlacer, FromSolutionEntitySelector, StaticTypedValueSelector,
    SimpleScoreDirector,
};

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

/// Calculates the score (number of constraint violations).
///
/// Constraints:
/// 1. No two queens on the same row
/// 2. No two queens on the same ascending diagonal
/// 3. No two queens on the same descending diagonal
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

fn get_queens(s: &NQueensSolution) -> &Vec<Queen> {
    &s.queens
}

fn get_queens_mut(s: &mut NQueensSolution) -> &mut Vec<Queen> {
    &mut s.queens
}

// Zero-erasure typed getter/setter for solution-level access
fn get_queen_row(s: &NQueensSolution, idx: usize) -> Option<i32> {
    s.queens.get(idx).and_then(|q| q.row)
}

fn set_queen_row(s: &mut NQueensSolution, idx: usize, v: Option<i32>) {
    if let Some(queen) = s.queens.get_mut(idx) {
        queen.row = v;
    }
}

/// Creates the solution descriptor with entity extractors.
fn create_descriptor() -> SolutionDescriptor {
    let extractor = Box::new(TypedEntityExtractor::new(
        "Queen",
        "queens",
        get_queens,
        get_queens_mut,
    ));

    let entity_desc = EntityDescriptor::new("Queen", TypeId::of::<Queen>(), "queens")
        .with_extractor(extractor);

    SolutionDescriptor::new("NQueensSolution", TypeId::of::<NQueensSolution>())
        .with_entity(entity_desc)
}

/// Creates a ScoreDirector using the score calculation.
fn create_score_director(
    solution: NQueensSolution,
) -> SimpleScoreDirector<NQueensSolution, fn(&NQueensSolution) -> SimpleScore> {
    let descriptor = create_descriptor();
    SimpleScoreDirector::with_calculator(solution, descriptor, calculate_score)
}

fn main() {
    println!("SolverForge N-Queens Example");
    println!("============================\n");

    // Create a 4-Queens problem (also works with larger n)
    let n = 4;
    let solution = NQueensSolution::new(n);

    println!("Problem: {} queens on a {}x{} board", n, n, n);
    println!("Queens are fixed to columns, solver will assign rows.\n");

    // Create the score director
    let director = create_score_director(solution);

    // Create value range (possible rows)
    let values: Vec<i32> = (0..n).collect();

    type NQueensMove = ChangeMove<NQueensSolution, i32>;

    // Create phases using factories
    // Phase 1: Construction heuristic to build initial solution
    let values1 = values.clone();
    let construction_factory = ConstructionPhaseFactory::<NQueensSolution, NQueensMove, _>::first_fit(
        move || {
            let es = Box::new(FromSolutionEntitySelector::new(0));
            let vs = Box::new(StaticTypedValueSelector::new(values1.clone()));
            Box::new(QueuedEntityPlacer::new(es, vs, get_queen_row, set_queen_row, 0, "row"))
        },
    );
    let construction_phase = construction_factory.create_phase();

    // Phase 2: Local search to improve the solution
    let values2 = values.clone();
    let local_search_factory = LocalSearchPhaseFactory::<NQueensSolution, NQueensMove, _>::hill_climbing(
        move || Box::new(ChangeMoveSelector::<NQueensSolution, i32>::simple(
            get_queen_row, set_queen_row, 0, "row", values2.clone()
        )),
    ).with_step_limit(100);
    let local_search = local_search_factory.create_phase();

    // Create and configure solver
    let mut solver = Solver::new(vec![construction_phase, local_search])
        .with_termination(Box::new(StepCountTermination::new(200)));

    println!("Solving with Construction Heuristic + Hill Climbing Local Search...\n");

    // Solve!
    let result = solver.solve_with_director(Box::new(director));

    // Display result
    result.print_board();

    let score = result.score().unwrap_or_else(|| calculate_score(&result));
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
    let director = create_score_director(solution);

    let values: Vec<i32> = (0..n).collect();
    let values1 = values.clone();
    let construction_factory = ConstructionPhaseFactory::<NQueensSolution, NQueensMove, _>::best_fit(
        move || {
            let es = Box::new(FromSolutionEntitySelector::new(0));
            let vs = Box::new(StaticTypedValueSelector::new(values1.clone()));
            Box::new(QueuedEntityPlacer::new(es, vs, get_queen_row, set_queen_row, 0, "row"))
        },
    );
    let construction_phase = construction_factory.create_phase();

    let values2 = values.clone();
    let local_search_factory = LocalSearchPhaseFactory::<NQueensSolution, NQueensMove, _>::hill_climbing(
        move || Box::new(ChangeMoveSelector::<NQueensSolution, i32>::simple(
            get_queen_row, set_queen_row, 0, "row", values2.clone()
        )),
    ).with_step_limit(500);
    let local_search = local_search_factory.create_phase();

    let mut solver = Solver::new(vec![construction_phase, local_search]);

    println!("Solving 8-Queens with Best Fit Construction + Hill Climbing...\n");

    let result = solver.solve_with_director(Box::new(director));
    result.print_board();

    let score = result.score().unwrap_or_else(|| calculate_score(&result));
    if score.is_feasible() {
        println!("\nSolution is OPTIMAL!");
    } else {
        println!("\nReached local optimum with {} conflicts.", -score.score());
    }

    println!("\nSolverForge solver is working!");
}
