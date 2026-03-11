//! Builder module for constructing solver components from configuration.
//!
//! Provides wiring between `SolverConfig` and the actual solver types.
//! All builders return concrete monomorphized enums — no `Box<dyn Trait>`.

pub mod acceptor;
pub mod basic_selector;
pub mod context;
pub mod forager;
pub mod list_selector;

pub use acceptor::{AcceptorBuilder, AnyAcceptor};
pub use basic_selector::{BasicLeafSelector, BasicMoveSelectorBuilder};
pub use context::{BasicContext, IntraDistanceAdapter, ListContext};
pub use forager::{AnyForager, ForagerBuilder};
pub use list_selector::{ListLeafSelector, ListMoveSelectorBuilder};
