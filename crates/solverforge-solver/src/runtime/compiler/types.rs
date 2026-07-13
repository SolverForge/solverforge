use std::fmt;

use solverforge_config::VariableTargetConfig;

use crate::builder::context::RuntimeListSlot;
use crate::builder::{RuntimeScalarSlot, RuntimeScalarSlotId};

/// One canonical scalar carrier and identity across selector compilation and
/// generic provider normalization.  No typed/dynamic selector tree may grow
/// beside this shared access boundary.
pub(crate) type CompiledScalarSlot<S> = RuntimeScalarSlot<S>;
pub(crate) type RuntimeSlotId = RuntimeScalarSlotId;

/// One physical list-slot carrier.  It is the counterpart of
/// [`CompiledScalarSlot`]: the future generic `ListAccess` kernel owns all
/// behavior, while the carrier only preserves native static dispatch versus
/// dynamic slot storage at the outer move payload boundary.
pub(crate) type CompiledListSlot<S, V, DM, IDM> = RuntimeListSlot<S, V, DM, IDM>;

/// Extension declaration family selected by a configured runtime phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeExtensionKind {
    Custom,
    Partitioned,
}

impl fmt::Display for RuntimeExtensionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Custom => f.write_str("custom"),
            Self::Partitioned => f.write_str("partitioned_search"),
        }
    }
}

/// A structural primitive required by one compiled graph node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeCapability {
    ScalarCandidates,
    ScalarNearbyValue,
    ScalarNearbyEntity,
    ScalarEntityOrder,
    ScalarValueOrder,
    ListSet,
    ListReverse,
    ListSublist,
    ListPrecedence,
    ListCrossPositionDistance,
    ListIntraPositionDistance,
    ListRoute,
    ListSavings,
}

impl fmt::Display for RuntimeCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::ScalarCandidates => "scalar candidate values",
            Self::ScalarNearbyValue => "nearby scalar value source",
            Self::ScalarNearbyEntity => "nearby scalar entity source",
            Self::ScalarEntityOrder => "scalar construction entity order",
            Self::ScalarValueOrder => "scalar construction value order",
            Self::ListSet => "direct list set",
            Self::ListReverse => "direct list reverse",
            Self::ListSublist => "direct list sublist mutation",
            Self::ListPrecedence => "list precedence metadata",
            Self::ListCrossPositionDistance => "cross-position list distance",
            Self::ListIntraPositionDistance => "intra-position list distance",
            Self::ListRoute => "route bundle (read/replace/depot/distance/feasible)",
            Self::ListSavings => "savings bundle (replace/depot/metric_class/distance/feasible)",
        };
        f.write_str(name)
    }
}

/// One precise non-routed compilation error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeCompileError {
    pub path: String,
    pub kind: RuntimeCompileErrorKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeCompileErrorKind {
    NoMatchingScalarSlot {
        target: VariableTargetConfig,
    },
    NoMatchingListSlot {
        target: VariableTargetConfig,
    },
    MissingCapability {
        slot: RuntimeSlotId,
        capability: RuntimeCapability,
    },
    AssignmentOwnedScalar {
        slot: RuntimeSlotId,
    },
    MissingScalarGroup {
        group_name: String,
    },
    MissingConflictRepairProvider,
    DuplicateProviderGroupName {
        group_name: String,
    },
    InvalidSlotIdentity {
        message: String,
    },
    EmptyUnion,
    InvalidCartesianArity {
        actual: usize,
    },
    PreviewUnsafeCartesianLeft,
    LocalSearchShape {
        message: String,
    },
    ConstructionShape {
        message: String,
    },
    ContextSeedMismatch {
        config_seed: Option<u64>,
        context_seed: Option<u64>,
    },
    MissingCustomExtensionName,
    MissingPartitionerName,
    UnregisteredTypedCustomExtension {
        name: String,
    },
    UnregisteredTypedPartitioner {
        name: String,
    },
    UnsupportedDynamicExtension {
        extension: RuntimeExtensionKind,
    },
}

impl fmt::Display for RuntimeCompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "runtime graph compilation at {}: ", self.path)?;
        match &self.kind {
            RuntimeCompileErrorKind::NoMatchingScalarSlot { target } => write!(
                f,
                "no scalar planning slot matches entity_class={:?} variable_name={:?}",
                target.entity_class, target.variable_name
            ),
            RuntimeCompileErrorKind::NoMatchingListSlot { target } => write!(
                f,
                "no list planning slot matches entity_class={:?} variable_name={:?}",
                target.entity_class, target.variable_name
            ),
            RuntimeCompileErrorKind::MissingCapability { slot, capability } => {
                write!(f, "{slot} does not provide required {capability}")
            }
            RuntimeCompileErrorKind::AssignmentOwnedScalar { slot } => write!(
                f,
                "{slot} is assignment-owned; use its named grouped scalar selector/construction"
            ),
            RuntimeCompileErrorKind::MissingScalarGroup { group_name } => {
                write!(f, "no immutable scalar-group binding named `{group_name}` exists")
            }
            RuntimeCompileErrorKind::MissingConflictRepairProvider => f.write_str(
                "no immutable conflict-repair provider binding exists for this selector",
            ),
            RuntimeCompileErrorKind::DuplicateProviderGroupName { group_name } => write!(
                f,
                "group `{group_name}` is declared by both a canonical scalar binding and the generic provider registry"
            ),
            RuntimeCompileErrorKind::InvalidSlotIdentity { message } => f.write_str(message),
            RuntimeCompileErrorKind::EmptyUnion => {
                f.write_str("union_move_selector must contain at least one child")
            }
            RuntimeCompileErrorKind::InvalidCartesianArity { actual } => write!(
                f,
                "cartesian_product_move_selector requires exactly two children, found {actual}"
            ),
            RuntimeCompileErrorKind::PreviewUnsafeCartesianLeft => f.write_str(
                "cartesian left child contains a score-during-move ruin selector and cannot be previewed",
            ),
            RuntimeCompileErrorKind::LocalSearchShape { message }
            | RuntimeCompileErrorKind::ConstructionShape { message } => f.write_str(message),
            RuntimeCompileErrorKind::ContextSeedMismatch {
                config_seed,
                context_seed,
            } => write!(
                f,
                "runtime extension context seed {context_seed:?} differs from authoritative solver config seed {config_seed:?}"
            ),
            RuntimeCompileErrorKind::MissingCustomExtensionName => {
                f.write_str("custom phase requires a non-empty `name`")
            }
            RuntimeCompileErrorKind::MissingPartitionerName => {
                f.write_str("partitioned_search requires a non-empty `partitioner` name")
            }
            RuntimeCompileErrorKind::UnregisteredTypedCustomExtension { name } => write!(
                f,
                "custom phase `{name}` was not registered by the typed runtime extension registry"
            ),
            RuntimeCompileErrorKind::UnregisteredTypedPartitioner { name } => write!(
                f,
                "partitioned_search partitioner `{name}` was not registered by the typed runtime extension registry"
            ),
            RuntimeCompileErrorKind::UnsupportedDynamicExtension { extension } => {
                write!(f, "dynamic runtime cannot register {extension} extensions")
            }
        }
    }
}

impl std::error::Error for RuntimeCompileError {}
