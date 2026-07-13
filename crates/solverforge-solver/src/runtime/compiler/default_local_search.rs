//! Capability-complete omitted local-search declarations.
//!
//! The historical builder default inspected only typed list slots, which made
//! a dynamic list model lose its whole default local-search neighborhood. The
//! immutable graph instead freezes one capability policy for both carriers.

use solverforge_config::{MoveSelectorConfig, UnionSelectionOrder, VariableTargetConfig};

use crate::builder::RuntimeScalarSlotId;

mod policy;
mod trace;

pub(super) use policy::{
    compile_default_local_search_components, compile_default_local_search_plan,
};
pub(super) use trace::{selection_order_label, selector_config_signature, slot_signature};

/// The exact default acceptor selected from frozen model capabilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DefaultLocalSearchAcceptorPolicy {
    LateAcceptance {
        history_size: usize,
    },
    DiversifiedLateAcceptance {
        history_size: usize,
    },
    SimulatedAnnealing {
        /// `f64::to_bits()` keeps this immutable declaration comparable.
        decay_rate_bits: u64,
        random_seed: Option<u64>,
    },
}

impl DefaultLocalSearchAcceptorPolicy {
    pub(crate) fn trace_label(self) -> &'static str {
        match self {
            Self::LateAcceptance { .. } => "late_acceptance",
            Self::DiversifiedLateAcceptance { .. } => "diversified_late_acceptance",
            Self::SimulatedAnnealing { .. } => "simulated_annealing",
        }
    }
}

/// The exact default forager selected from frozen model capabilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DefaultLocalSearchForagerPolicy {
    AcceptedCount { limit: usize },
    FirstLastStepScoreImproving { accepted_count_limit: Option<usize> },
}

impl DefaultLocalSearchForagerPolicy {
    pub(crate) fn trace_label(self) -> &'static str {
        match self {
            Self::AcceptedCount { .. } => "accepted_count",
            Self::FirstLastStepScoreImproving { .. } => "first_last_step_score_improving",
        }
    }
}

/// Components usable by an explicit local-search phase that omits only its
/// acceptor or forager.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DefaultLocalSearchComponents {
    pub acceptor: DefaultLocalSearchAcceptorPolicy,
    pub forager: DefaultLocalSearchForagerPolicy,
}

/// The default selector family, in canonical execution order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DefaultLocalSearchSelectorFamily {
    ListPrecedence,
    ListPermute,
    NearbyListChange,
    PlainListChange,
    NearbyListSwap,
    PlainListSwap,
    SublistChange,
    SublistSwap,
    ListReverse,
    NearbyKOpt,
    UnboundedKOpt,
    ListRuin,
    NearbyScalarChange,
    NearbyScalarSwap,
    ScalarChange,
    ScalarSwap,
    GroupedScalar,
    CompoundConflictRepair,
}

impl DefaultLocalSearchSelectorFamily {
    pub(crate) fn trace_label(self) -> &'static str {
        match self {
            Self::ListPrecedence => "list_precedence",
            Self::ListPermute => "list_permute",
            Self::NearbyListChange => "nearby_list_change",
            Self::PlainListChange => "list_change",
            Self::NearbyListSwap => "nearby_list_swap",
            Self::PlainListSwap => "list_swap",
            Self::SublistChange => "sublist_change",
            Self::SublistSwap => "sublist_swap",
            Self::ListReverse => "list_reverse",
            Self::NearbyKOpt => "nearby_k_opt",
            Self::UnboundedKOpt => "unbounded_k_opt",
            Self::ListRuin => "list_ruin",
            Self::NearbyScalarChange => "nearby_scalar_change",
            Self::NearbyScalarSwap => "nearby_scalar_swap",
            Self::ScalarChange => "scalar_change",
            Self::ScalarSwap => "scalar_swap",
            Self::GroupedScalar => "grouped_scalar",
            Self::CompoundConflictRepair => "compound_conflict_repair",
        }
    }
}

/// Capability row selected for one leaf. Reduced-capability rows are named
/// so provenance cannot claim that an unavailable metric was used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DefaultSelectorCapabilityPolicy {
    DeclaredCapabilities,
    PlainListChangeWithoutCrossPositionDistance,
    PlainListSwapWithoutCrossPositionDistance,
    UnboundedKOptWithoutIntraPositionDistance,
}

impl DefaultSelectorCapabilityPolicy {
    pub(crate) fn trace_label(self) -> &'static str {
        match self {
            Self::DeclaredCapabilities => "declared_capabilities",
            Self::PlainListChangeWithoutCrossPositionDistance => {
                "plain_list_change_without_cross_position_distance"
            }
            Self::PlainListSwapWithoutCrossPositionDistance => {
                "plain_list_swap_without_cross_position_distance"
            }
            Self::UnboundedKOptWithoutIntraPositionDistance => {
                "unbounded_k_opt_without_intra_position_distance"
            }
        }
    }
}

/// One frozen selector declaration and the exact slots it receives.
#[derive(Clone, Debug)]
pub(crate) struct DefaultLocalSearchSelectorDeclaration {
    pub family: DefaultLocalSearchSelectorFamily,
    pub capability_policy: DefaultSelectorCapabilityPolicy,
    pub config: MoveSelectorConfig,
    pub slots: Vec<RuntimeScalarSlotId>,
}

/// The immutable omitted-local-search policy. The executor will instantiate
/// its leaves from the exact declaration-order runtime bindings; this type
/// intentionally owns no cloned slot carriers.
#[derive(Clone, Debug)]
pub(crate) struct DefaultLocalSearchPlan {
    pub components: DefaultLocalSearchComponents,
    pub selection_order: UnionSelectionOrder,
    pub selectors: Vec<DefaultLocalSearchSelectorDeclaration>,
}

pub(super) fn target_for(slot: &RuntimeScalarSlotId) -> VariableTargetConfig {
    VariableTargetConfig {
        entity_class: Some(slot.entity_class.to_string()),
        variable_name: Some(slot.variable_name.to_string()),
    }
}
