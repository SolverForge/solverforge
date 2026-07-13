mod candidate_metric;
mod conflict_repair;
mod list;
pub(crate) mod list_access;
mod model;
mod model_resolution;
mod provider;
mod runtime_list;
mod runtime_list_binding;
mod runtime_list_debug;
mod runtime_list_metadata_policy;
mod runtime_list_policy;
mod runtime_list_route;
mod runtime_list_route_policy;
mod runtime_list_source;
mod scalar;
mod scalar_access;

#[cfg(test)]
mod runtime_list_distance_tests;
#[cfg(test)]
mod runtime_list_dynamic_precedence_tests;
#[cfg(test)]
mod runtime_list_metric_policy_tests;
#[cfg(test)]
mod runtime_list_route_tests;

pub use candidate_metric::{
    RuntimeCandidateMetric, RuntimeCandidateMetricBinding, RuntimeCandidateMetricRegistry,
};
pub use conflict_repair::{ConflictRepair, RepairCandidate, RepairLimits, RepairProvider};
pub use list::{usize_element_source_key, IntraDistanceAdapter, ListVariableSlot};
pub use model::{RuntimeModel, VariableSlot};
pub use provider::{
    ProviderNormalizationState, ProviderReasonArena, ProviderReasonId, ProviderResolutionError,
    RawProviderCandidate, RawProviderEdit, ResolvedProviderCandidate, ResolvedProviderEdit,
    RuntimeConflictRepairProviderBinding, RuntimeHostCompoundProvider,
    RuntimeHostProviderErrorBoundary, RuntimeProviderHandle, RuntimeProviderLimits,
    RuntimeProviderRegistry, RuntimeProviderSlotResolver, RuntimeScalarGroupProviderBinding,
    StaticConflictRepairProviderBinding, StaticScalarGroupProviderBinding,
};
pub(crate) use runtime_list::{RuntimeListElement, RuntimeListSlot};
pub(crate) use runtime_list_policy::{
    ConstructionOrderPolicy, OwnershipPolicy, PrecedencePolicy, RouteFeasibilityPolicy,
    RouteReadPolicy, RouteReplacePolicy, SavingsMetricClassPolicy,
};
pub(crate) use runtime_list_source::{
    bind_runtime_list_source, unassigned_from_current_assignment, ListConstructionKernelError,
    ListSourceAccess, RuntimeListSourceIndex, SourceElement,
};
pub use scalar::{
    bind_scalar_groups, ConstructionEntityOrderKey, ConstructionValueOrderKey,
    NearbyEntityDistanceMeter, NearbyValueDistanceMeter, ScalarAssignmentBinding, ScalarCandidate,
    ScalarCandidateProvider, ScalarCandidateValues, ScalarEdit, ScalarGetter, ScalarGroupBinding,
    ScalarGroupBindingKind, ScalarGroupLimits, ScalarGroupMemberBinding, ScalarSetter,
    ScalarVariableSlot, ValueSource,
};
pub use scalar_access::{
    RuntimeScalarEdit, RuntimeScalarSlot, RuntimeScalarSlotId, ScalarAccessCapability,
};
