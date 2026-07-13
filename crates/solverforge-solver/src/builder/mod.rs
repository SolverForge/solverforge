/* Builder module for constructing solver components from configuration.

Provides wiring between `SolverConfig` and the actual solver types.
All builders return concrete monomorphized enums — no `Box<dyn Trait>`.
*/

pub mod acceptor;
pub mod context;
pub mod forager;
pub mod search;
pub(crate) mod selector;

pub use acceptor::{AcceptorBuilder, AnyAcceptor};
pub use context::{
    bind_scalar_groups, usize_element_source_key, ConflictRepair, IntraDistanceAdapter,
    ListVariableSlot, RepairCandidate, RepairLimits, RuntimeCandidateMetric,
    RuntimeCandidateMetricBinding, RuntimeCandidateMetricRegistry, RuntimeHostCompoundProvider,
    RuntimeHostProviderErrorBoundary, RuntimeModel, RuntimeProviderHandle, RuntimeProviderLimits,
    RuntimeProviderRegistry, RuntimeScalarEdit, RuntimeScalarSlot, RuntimeScalarSlotId,
    ScalarAccessCapability, ScalarAssignmentBinding, ScalarCandidate, ScalarEdit,
    ScalarGroupBinding, ScalarGroupBindingKind, ScalarGroupLimits, ScalarGroupMemberBinding,
    ScalarVariableSlot, StaticConflictRepairProviderBinding, StaticScalarGroupProviderBinding,
    ValueSource, VariableSlot,
};
pub use forager::{AnyForager, ForagerBuilder};
pub use search::{
    local_search, CustomSearchPhase, NoDynamicExtensions, NoTypedExtensions,
    RuntimeExtensionPolicy, RuntimeExtensionRegistry, Search, SearchContext,
};
