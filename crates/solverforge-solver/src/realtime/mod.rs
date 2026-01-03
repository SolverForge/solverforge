//! Real-time planning support.
//!
//! Allows submitting problem changes while the solver is running. Changes are
//! processed at step boundaries to maintain solver consistency.
//!
//! # Overview
//!
//! Real-time planning enables dynamic updates to the problem during solving:
//! - Add new entities (e.g., new orders, tasks, employees)
//! - Remove entities (e.g., cancelled orders)
//! - Update entity properties (e.g., deadline changes)
//! - Modify problem facts (e.g., new constraints)
//!
//! # Example
//!
//! ```
//! use solverforge_solver::realtime::ProblemChange;
//! use solverforge_scoring::ScoreDirector;
//! use solverforge_core::domain::PlanningSolution;
//! use solverforge_core::score::SimpleScore;
//!
//! #[derive(Clone, Debug)]
//! struct Task { id: usize, priority: Option<i32> }
//!
//! #[derive(Clone, Debug)]
//! struct Schedule {
//!     tasks: Vec<Task>,
//!     score: Option<SimpleScore>,
//! }
//!
//! impl PlanningSolution for Schedule {
//!     type Score = SimpleScore;
//!     fn score(&self) -> Option<Self::Score> { self.score }
//!     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
//! }
//!
//! // Create a problem change that adds a new task
//! #[derive(Debug)]
//! struct AddTask { id: usize }
//!
//! impl ProblemChange<Schedule> for AddTask {
//!     fn apply(&self, score_director: &mut dyn ScoreDirector<Schedule>) {
//!         let task = Task { id: self.id, priority: None };
//!         score_director.working_solution_mut().tasks.push(task);
//!         score_director.trigger_variable_listeners();
//!     }
//! }
//! ```

mod problem_change;
mod solver_handle;

pub use problem_change::{BoxedProblemChange, ClosureProblemChange, ProblemChange};
pub use solver_handle::{ProblemChangeReceiver, ProblemChangeResult, SolverHandle};
