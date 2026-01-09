//! Phase factory for creating phases from configuration.
//!
//! Phase factories create fresh phase instances for each solve, ensuring
//! clean state between solves. This is essential because phases maintain
//! internal state (like step counters, tabu lists, or temperature values)
//! that must be reset for each new solve.
//!
//! # Overview
//!
//! This module provides two main factories:
//!
//! - [`ConstructionPhaseFactory`]: Creates construction heuristic phases
//! - [`LocalSearchPhaseFactory`]: Creates local search phases
//!
//! # Usage Pattern
//!
//! ```
//! use solverforge_solver::manager::{LocalSearchPhaseFactory, SolverPhaseFactory, LocalSearchType};
//! use solverforge_solver::heuristic::{Move, MoveSelector};
//! use solverforge_solver::heuristic::selector::ChangeMoveSelector;
//! use solverforge_solver::heuristic::r#move::ChangeMove;
//! use solverforge_core::domain::PlanningSolution;
//! use solverforge_core::score::SimpleScore;
//!
//! #[derive(Clone)]
//! struct Sol { values: Vec<Option<i32>>, score: Option<SimpleScore> }
//!
//! impl PlanningSolution for Sol {
//!     type Score = SimpleScore;
//!     fn score(&self) -> Option<Self::Score> { self.score }
//!     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
//! }
//!
//! fn get_v(s: &Sol, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
//! fn set_v(s: &mut Sol, idx: usize, v: Option<i32>) { if let Some(x) = s.values.get_mut(idx) { *x = v; } }
//!
//! type TestMove = ChangeMove<Sol, i32>;
//!
//! // Create a local search phase factory with tabu search
//! let factory = LocalSearchPhaseFactory::<Sol, TestMove, _>::tabu_search(7, || {
//!     Box::new(ChangeMoveSelector::<Sol, i32>::simple(get_v, set_v, 0, "value", vec![1, 2, 3]))
//! });
//!
//! // Each call to create_phase() returns a fresh phase with clean state
//! let phase1 = factory.create_phase();
//! let phase2 = factory.create_phase(); // Independent of phase1
//! ```

mod construction;
mod list_construction;
mod local_search;

pub use construction::ConstructionPhaseFactory;
pub use list_construction::ListConstructionPhaseBuilder;
pub use local_search::{KOptPhaseBuilder, LocalSearchPhaseFactory};
