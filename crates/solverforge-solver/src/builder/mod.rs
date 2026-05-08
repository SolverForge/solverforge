/* Builder module for constructing solver components from configuration.

Provides wiring between `SolverConfig` and the actual solver types.
All builders return concrete monomorphized enums — no `Box<dyn Trait>`.
*/

pub mod acceptor;
pub mod context;
pub mod forager;
mod list_selector;
mod scalar_selector;
pub mod selector;

pub use acceptor::{AcceptorBuilder, AnyAcceptor};
pub use context::{
    bind_coverage_groups, bind_scalar_groups, ConflictRepair, CoverageGroupBinding,
    IntraDistanceAdapter, ListVariableSlot, RepairCandidate, RepairLimits, RuntimeModel,
    ScalarCandidate, ScalarEdit, ScalarGroupBinding, ScalarGroupLimits, ScalarGroupMemberBinding,
    ScalarVariableSlot, ValueSource, VariableSlot,
};
pub use forager::{AnyForager, ForagerBuilder};
pub use selector::{
    build_local_search, build_move_selector, build_vnd, LocalSearch, Neighborhood,
    NeighborhoodLeaf, NeighborhoodMove, Selector, Vnd,
};
