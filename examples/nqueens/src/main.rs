//! N-Queens Example
//!
//! Place N queens on an N×N board so no two threaten each other.

use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;
use solverforge::__internal::TypedScoreDirector;
use rand::Rng;

#[planning_entity]
pub struct Queen {
    #[planning_id]
    pub id: i32,
    pub column: i32,
    #[planning_variable(allows_unassigned = true)]
    pub row: Option<i32>,
}

#[planning_solution]
pub struct NQueensSolution {
    pub n: i32,
    #[planning_entity_collection]
    pub queens: Vec<Queen>,
    #[planning_score]
    pub score: Option<SimpleScore>,
}

impl NQueensSolution {
    pub fn new(n: i32) -> Self {
        Self {
            n,
            queens: (0..n).map(|i| Queen { id: i, column: i, row: None }).collect(),
            score: None,
        }
    }

    pub fn print_board(&self) {
        println!("\n{}-Queens (Score: {:?}):", self.n, self.score);
        for row in 0..self.n {
            print!("|");
            for col in 0..self.n {
                let has_queen = self.queens.iter().any(|q| q.column == col && q.row == Some(row));
                print!("{}", if has_queen { "Q|" } else { " |" });
            }
            println!();
        }
    }
}

fn define_constraints() -> impl ConstraintSet<NQueensSolution, SimpleScore> {
    let factory = ConstraintFactory::<NQueensSolution, SimpleScore>::new();

    let row_conflict = factory
        .clone()
        .for_each_unique_pair(
            |s: &NQueensSolution| s.queens.as_slice(),
            joiner::equal(|q: &Queen| q.row),
        )
        .filter(|a: &Queen, b: &Queen| a.row.is_some() && b.row.is_some())
        .penalize(SimpleScore::of(1))
        .as_constraint("Row conflict");

    let asc_diagonal = factory
        .clone()
        .for_each_unique_pair(
            |s: &NQueensSolution| s.queens.as_slice(),
            joiner::equal(|q: &Queen| q.row.map(|r| r - q.column)),
        )
        .filter(|a: &Queen, b: &Queen| a.row.is_some() && b.row.is_some())
        .penalize(SimpleScore::of(1))
        .as_constraint("Ascending diagonal");

    let desc_diagonal = factory
        .for_each_unique_pair(
            |s: &NQueensSolution| s.queens.as_slice(),
            joiner::equal(|q: &Queen| q.row.map(|r| r + q.column)),
        )
        .filter(|a: &Queen, b: &Queen| a.row.is_some() && b.row.is_some())
        .penalize(SimpleScore::of(1))
        .as_constraint("Descending diagonal");

    (row_conflict, asc_diagonal, desc_diagonal)
}

fn solve(solution: NQueensSolution) -> NQueensSolution {
    let n = solution.n;
    let mut director = TypedScoreDirector::new(solution, define_constraints());
    let mut rng = rand::thread_rng();

    // Construction: round-robin row assignment
    for i in 0..director.working_solution().queens.len() {
        director.before_variable_changed(i);
        director.working_solution_mut().queens[i].row = Some((i as i32) % n);
        director.after_variable_changed(i);
    }

    // Local search: hill climbing
    let mut score = director.get_score();
    for _ in 0..1000 {
        if score.is_feasible() { break; }

        let idx = rng.gen_range(0..director.working_solution().queens.len());
        let old = director.working_solution().queens[idx].row;
        let new = Some(rng.gen_range(0..n));

        director.before_variable_changed(idx);
        director.working_solution_mut().queens[idx].row = new;
        director.after_variable_changed(idx);

        let new_score = director.get_score();
        if new_score >= score {
            score = new_score;
        } else {
            director.before_variable_changed(idx);
            director.working_solution_mut().queens[idx].row = old;
            director.after_variable_changed(idx);
        }
    }

    let mut result = director.clone_working_solution();
    result.score = Some(score);
    result
}

fn main() {
    println!("SolverForge N-Queens Example\n");

    for n in [4, 8] {
        println!("Solving {}-Queens...", n);
        let result = solve(NQueensSolution::new(n));
        result.print_board();
        println!("{}\n", if result.score.map_or(false, |s| s.is_feasible()) { "OPTIMAL!" } else { "Local optimum." });
    }
}
