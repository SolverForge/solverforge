/* Builder module for constructing solver components from configuration.

Provides wiring between `SolverConfig` and the actual solver types.
All builders return concrete monomorphized enums — no `Box<dyn Trait>`.
*/

pub mod acceptor;
pub mod context;
pub mod forager;
pub mod list_selector;
pub mod standard_selector;

pub use acceptor::{AcceptorBuilder, AnyAcceptor};
pub use context::{IntraDistanceAdapter, ListContext, StandardContext};
pub use forager::{AnyForager, ForagerBuilder};
pub use list_selector::{ListLeafSelector, ListMoveSelectorBuilder};
pub use standard_selector::{StandardLeafSelector, StandardMoveSelectorBuilder};
