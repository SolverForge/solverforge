//! Zero-erasure phase functions for fluent API.
//!
//! These functions work directly with `TypedScoreDirector<S, C>` instead of
//! trait objects, enabling full monomorphization and zero type erasure.
//!
//! # Example
//!
//! ```ignore
//! use solverforge_solver::phase::fluent::{list_construction_phase, two_opt_phase};
//!
//! // Build director with typed constraints
//! let mut director = TypedScoreDirector::new(solution, constraints);
//!
//! // Construction: assign all unassigned elements
//! list_construction_phase(&mut director);
//!
//! // Local search: improve via 2-opt moves
//! two_opt_phase(&mut director, time_limit, &terminate);
//! ```

mod construction;
mod local_search;

pub use construction::list_construction_phase;
pub use local_search::two_opt_phase;
