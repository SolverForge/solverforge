pub(crate) mod candidate_metric;
mod conflict_repair;
pub(crate) mod list;
pub(crate) mod list_access;
pub(crate) mod model;
mod model_resolution;
pub(crate) mod provider;
mod runtime_list;
mod runtime_list_binding;
mod runtime_list_debug;
mod runtime_list_metadata_policy;
mod runtime_list_policy;
mod runtime_list_route;
mod runtime_list_route_policy;
mod runtime_list_source;
pub(crate) mod scalar;
pub(crate) mod scalar_access;

#[cfg(test)]
mod runtime_list_distance_tests;
#[cfg(test)]
mod runtime_list_dynamic_precedence_tests;
#[cfg(test)]
mod runtime_list_metric_policy_tests;
#[cfg(test)]
mod runtime_list_route_tests;

pub(crate) use candidate_metric::{RuntimeCandidateMetricBinding, RuntimeCandidateMetricRegistry};
pub(crate) use conflict_repair::ConflictRepair;
pub(crate) use list::ListVariableSlot;
pub(crate) use model::{RuntimeModel, VariableSlot};
pub(crate) use provider::{
    ProviderNormalizationState, ProviderReasonArena, ProviderReasonId, RawProviderCandidate,
    ResolvedProviderCandidate, RuntimeProviderHandle, RuntimeProviderLimits,
    RuntimeProviderRegistry,
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
pub(crate) use scalar::{
    ScalarAssignmentBinding, ScalarCandidate, ScalarCandidateProvider, ScalarGroupBinding,
    ScalarGroupBindingKind, ScalarGroupLimits, ScalarVariableSlot, ValueSource,
};
pub(crate) use scalar_access::{
    RuntimeScalarEdit, RuntimeScalarSlot, RuntimeScalarSlotId, ScalarAccessCapability,
};
