use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, LocalSearchConfig, MoveSelectorConfig,
    PartitionedSearchConfig, SelectionOrder, SolverConfig, UnionSelectionOrder, UnionWeighting,
};

use crate::builder::{
    RuntimeCandidateMetricBinding, RuntimeProviderHandle, RuntimeScalarSlotId, ScalarGroupBinding,
    SearchContext,
};
use crate::descriptor::ResolvedVariableBinding;
use crate::phase::construction::{ScalarConstructionSchedule, ScalarOrMixedSlotOrder};

use super::defaults::DefaultRuntimeBindings;
use super::types::{CompiledListSlot, CompiledScalarSlot};

/// Scalar leaf families whose execution will be shared by typed and dynamic
/// `ScalarAccess` kernels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScalarLeafKind {
    Change,
    Swap,
    NearbyChange,
    NearbySwap,
    PillarChange,
    PillarSwap,
    RuinRecreate,
}

/// List leaf families whose execution will be shared by typed and dynamic
/// `ListAccess` kernels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListLeafKind {
    Change,
    NearbyChange,
    Swap,
    Permute,
    Precedence,
    NearbySwap,
    SublistChange,
    SublistSwap,
    Reverse,
    KOpt,
    Ruin,
}

/// Exhaustive compiled list-construction family.
///
/// The executor must match this discriminator directly. In particular,
/// Clarke-Wright is never a cheapest-insertion alias: it owns its savings,
/// merge, and completion candidate trace semantics before any separately
/// configured K-opt phase runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListConstructionKind {
    RoundRobin,
    CheapestInsertion,
    RegretInsertion,
    ClarkeWright,
    KOpt,
}

impl ListConstructionKind {
    pub(super) fn from_heuristic(heuristic: ConstructionHeuristicType) -> Option<Self> {
        match heuristic {
            ConstructionHeuristicType::ListRoundRobin => Some(Self::RoundRobin),
            ConstructionHeuristicType::ListCheapestInsertion => Some(Self::CheapestInsertion),
            ConstructionHeuristicType::ListRegretInsertion => Some(Self::RegretInsertion),
            ConstructionHeuristicType::ListClarkeWright => Some(Self::ClarkeWright),
            ConstructionHeuristicType::ListKOpt => Some(Self::KOpt),
            _ => None,
        }
    }
}

/// Immutable binding for one source inside the common provider cursor.
///
/// Compilation never opens/pulls a provider.  The cursor receives only these
/// frozen handles, so `limited(0)`, unreached union branches, and Cartesian
/// right branches after an impossible left preview cannot observe callback
/// delivery.  A static Rust function and a host callback therefore share one
/// cursor/store/normalizer while retaining their documented ordering policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProviderBindingPlan {
    pub handle: RuntimeProviderHandle,
    /// Immutable source-schema provenance; never substitute a filtered vector
    /// position when emitting trace/provenance.
    pub declared_schema_index: usize,
    pub allowed_slots: Vec<RuntimeScalarSlotId>,
    pub policy: ProviderBindingPolicy,
    pub candidate_contract: ProviderCandidateContract,
}

/// Source-specific behavior interpreted by the one future provider cursor.
/// These variants are data, not selector implementations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProviderBindingPolicy {
    /// Host callback group: normalize/dedupe, take the callback cap (explicit
    /// zero clamps to one), then rotate the resulting move order.
    CallbackGroup {
        rotation_seed_salt: u64,
        pull_timing: ProviderPullTiming,
    },
    /// Existing Rust candidate group: pull/open behavior and rotate-before-
    /// normalization semantics remain explicit while using the same cursor.
    StaticGroup {
        rotation_seed_salt: u64,
        declared_max_moves_per_step: Option<usize>,
        pull_timing: ProviderPullTiming,
    },
    /// Host repair: rotate the full declaration vector before filtering,
    /// invoke a matching multi-constraint provider at most once, and reset
    /// deduplication for every provider result.
    CallbackRepair {
        rotation_seed_salt: u64,
        pull_timing: ProviderPullTiming,
    },
    /// Existing Rust repair: rotate configured constraints, matching provider
    /// indexes, and returned specs with the historic salts; dedup globally
    /// across its configured repair stream.
    StaticRepair {
        constraint_rotation_seed_salt: u64,
        provider_rotation_seed_salt: u64,
        spec_rotation_seed_salt: u64,
        pull_timing: ProviderPullTiming,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProviderPullTiming {
    /// Generic host callbacks are pulled only once the first reachable cursor
    /// `next_candidate` needs them.
    FirstReachableNext,
    /// Native Rust providers open eagerly; callback providers pull only when
    /// their first candidate is reached.
    OpenCursor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ProviderSchedule {
    Group {
        value_candidate_limit: Option<usize>,
        requested_max_moves_per_step: Option<usize>,
    },
    Repair {
        constraints: Vec<String>,
        max_matches_per_step: usize,
        max_repairs_per_match: usize,
        max_moves_per_step: usize,
        include_soft_matches: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CompiledProviderPlan {
    /// The existing dynamic public contract prefixes the tabu/ownership
    /// signature with this kind. Callback reason is deliberately excluded.
    pub move_kind: ProviderMoveKind,
    pub schedule: ProviderSchedule,
    pub bindings: Vec<ProviderBindingPlan>,
}

/// Stable provider-family discriminator retained in move/tabu ownership.
/// Values match the public wrapper's existing dynamic compound signatures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub(crate) enum ProviderMoveKind {
    Grouped = 1,
    ConflictRepair = 2,
    CompoundConflictRepair = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProviderCandidateContract {
    pub reason_storage: ProviderReasonStorage,
    pub deduplication: ProviderCandidateDeduplication,
    pub tabu_identity: ProviderTabuIdentity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProviderReasonStorage {
    /// Provider labels are interned into the solve-owned reason arena when
    /// raw callback output crosses the normalization boundary. Candidate moves
    /// thereafter carry this compact ID, never an `Arc<str>`.
    PerRunInternedId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProviderCandidateDeduplication {
    /// Same ordered edits with a different reason are distinct callback
    /// candidates, exactly as Python's existing declaration contract.
    PerProviderReasonAndOrderedEdits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProviderTabuIdentity {
    /// Tabu keys contain the provider-kind prefix and ordered edit
    /// targets/values.  User callback labels never affect seeded randomness,
    /// but the same edits emitted by Grouped, ConflictRepair, and
    /// CompoundConflictRepair remain distinct move families.
    ProviderKindAndOrderedEdits,
}

pub(super) const GROUP_PROVIDER_ROTATION_SALT: u64 = 0xC0A1_E5CE_DA7A_0001;
pub(super) const REPAIR_PROVIDER_ROTATION_SALT: u64 = 0xC0AF_11C7_DA7A_0001;
pub(super) const STATIC_GROUP_PROVIDER_ROTATION_SALT: u64 = 0xC0A1_E5CE_AAA0_0001;
pub(super) const STATIC_REPAIR_CONSTRAINT_ROTATION_SALT: u64 = 0xC0AF_11C7_0000_0001;
pub(super) const STATIC_REPAIR_PROVIDER_ROTATION_SALT: u64 = 0xC0AF_11C7_0000_0002;
pub(super) const STATIC_REPAIR_SPEC_ROTATION_SALT: u64 = 0xC0AF_11C7_0000_0003;

pub(super) const PYTHON_PROVIDER_CANDIDATE_CONTRACT: ProviderCandidateContract =
    ProviderCandidateContract {
        reason_storage: ProviderReasonStorage::PerRunInternedId,
        deduplication: ProviderCandidateDeduplication::PerProviderReasonAndOrderedEdits,
        tabu_identity: ProviderTabuIdentity::ProviderKindAndOrderedEdits,
    };

/// Recursive immutable selector graph.  `Limited`, `Union`, and `Cartesian`
/// are true recursive nodes so nested Cartesian graphs work identically for
/// acceptor-forager local search and VND neighborhoods.
#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "compiled selector nodes are an immutable value-owned graph"
)]
pub(crate) enum CompiledSelectorNode<S, V, DM, IDM> {
    Scalar {
        kind: ScalarLeafKind,
        config: MoveSelectorConfig,
        candidate_order: SelectionOrder,
        candidate_metric: Option<RuntimeCandidateMetricBinding<S>>,
        slots: Vec<CompiledScalarSlot<S>>,
    },
    List {
        kind: ListLeafKind,
        config: MoveSelectorConfig,
        candidate_order: SelectionOrder,
        candidate_metric: Option<RuntimeCandidateMetricBinding<S>>,
        slots: Vec<CompiledListSlot<S, V, DM, IDM>>,
    },
    GroupedScalar {
        config: MoveSelectorConfig,
        candidate_order: SelectionOrder,
        candidate_metric: Option<RuntimeCandidateMetricBinding<S>>,
        group_index: usize,
        group: ScalarGroupBinding<S>,
    },
    /// Every non-assignment group/repair source uses this one provider-node
    /// shape.  The eventual cursor dispatches only the frozen handle/policy;
    /// it does not choose a typed versus callback selector tree.
    Provider {
        config: MoveSelectorConfig,
        candidate_order: SelectionOrder,
        candidate_metric: Option<RuntimeCandidateMetricBinding<S>>,
        plan: CompiledProviderPlan,
    },
    Limited {
        selected_count_limit: usize,
        selector: Box<Self>,
    },
    Union {
        selection_order: UnionSelectionOrder,
        weighting: UnionWeighting,
        weights: Vec<u64>,
        children: Vec<Self>,
    },
    Cartesian {
        require_hard_improvement: bool,
        left: Box<Self>,
        right: Box<Self>,
    },
}

impl<S, V, DM, IDM> CompiledSelectorNode<S, V, DM, IDM> {
    pub(super) fn requires_score_during_move(&self) -> bool {
        match self {
            Self::Scalar { kind, .. } => matches!(kind, ScalarLeafKind::RuinRecreate),
            Self::List { kind, .. } => matches!(kind, ListLeafKind::Ruin),
            Self::GroupedScalar { .. } | Self::Provider { .. } => false,
            Self::Limited { selector, .. } => selector.requires_score_during_move(),
            Self::Union { children, .. } => children.iter().any(Self::requires_score_during_move),
            Self::Cartesian { left, right, .. } => {
                left.requires_score_during_move() || right.requires_score_during_move()
            }
        }
    }

    pub(super) fn contains_provider(&self) -> bool {
        match self {
            Self::Provider { .. } => true,
            Self::Limited { selector, .. } => selector.contains_provider(),
            Self::Union { children, .. } => children.iter().any(Self::contains_provider),
            Self::Cartesian { left, right, .. } => {
                left.contains_provider() || right.contains_provider()
            }
            Self::Scalar { .. } | Self::List { .. } | Self::GroupedScalar { .. } => false,
        }
    }
}

#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "compiled construction stays value-owned through preparation"
)]
pub(crate) enum CompiledConstruction<S, V, DM, IDM> {
    ScalarOrMixed {
        config: ConstructionHeuristicConfig,
        /// Frozen selection semantics. The executor must consume this
        /// directly instead of rediscovering typed/dynamic shape from a
        /// model or solution.
        schedule: ScalarConstructionSchedule,
        scalar_slots: Vec<CompiledScalarSlot<S>>,
        list_slots: Vec<CompiledListSlot<S, V, DM, IDM>>,
        /// Original declaration order plus the construction-frontier slot
        /// identity for each carrier. Scalar and list payloads are stored in
        /// compact separate vectors, but their execution order is never
        /// reconstructed from those vectors.
        slot_order: Vec<ScalarOrMixedSlotOrder>,
    },
    List {
        kind: ListConstructionKind,
        config: ConstructionHeuristicConfig,
        slots: Vec<CompiledListSlot<S, V, DM, IDM>>,
    },
    GroupedScalar {
        config: ConstructionHeuristicConfig,
        group_index: usize,
        group: ScalarGroupBinding<S>,
        /// Descriptor-resolved scalar construction bindings are frozen with
        /// the group.  The runtime executor must not rediscover them from a
        /// descriptor while constructing a candidate group.
        scalar_bindings: Vec<ResolvedVariableBinding<S>>,
    },
}

#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "compiled selector declarations stay value-owned"
)]
pub(crate) enum CompiledAcceptorForagerSelector<S, V, DM, IDM> {
    /// A user-declared selector compiled directly into the immutable graph.
    Explicit(CompiledSelectorNode<S, V, DM, IDM>),
    /// This phase intentionally consumes `DefaultRuntimeBindings`' one
    /// capability-resolved declaration. It is not an absent selector or a
    /// late-selected alternative.
    OmittedDefault,
}

#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "compiled local search is an immutable value-owned declaration"
)]
pub(crate) enum CompiledLocalSearch<S, V, DM, IDM> {
    AcceptorForager {
        config: LocalSearchConfig,
        selector: CompiledAcceptorForagerSelector<S, V, DM, IDM>,
    },
    VariableNeighborhoodDescent {
        config: LocalSearchConfig,
        neighborhoods: Vec<CompiledSelectorNode<S, V, DM, IDM>>,
    },
}

#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "compiled phases remain one value-owned execution graph"
)]
pub(crate) enum CompiledRuntimePhase<S, V, DM, IDM> {
    Construction(CompiledConstruction<S, V, DM, IDM>),
    LocalSearch(CompiledLocalSearch<S, V, DM, IDM>),
    Extension(CompiledRuntimeExtension),
    /// The omitted-phase profile is deliberately a first-class, per-solve
    /// graph node.  Its construction children depend on the current
    /// solution's unassigned rows and route content; its optional default
    /// local-search child depends on the effective solver termination.  A
    /// future instantiator resolves it from this compiled model and records
    /// that exact result for candidate trace provenance.
    DefaultRuntime,
}

/// Immutable declaration of one typed extension phase.
///
/// It contains only the selected registry name and frozen configuration.
/// Concrete extension builders are deliberately deferred to per-solve graph
/// execution so compilation has no callback, allocation, or phase side effect.
#[derive(Clone, Debug)]
pub(crate) enum CompiledRuntimeExtension {
    Custom {
        name: String,
    },
    Partitioned {
        name: String,
        config: PartitionedSearchConfig,
    },
}

/// Immutable per-config graph.  It deliberately contains no current solution,
/// callback view, thread-local state, or fresh selector cursor.  Those are
/// created only by `instantiate()` at the future atomic cutover.
pub(crate) struct CompiledRuntimeGraph<S, V, DM, IDM, E>
where
    S: solverforge_core::domain::PlanningSolution,
{
    /// One descriptor-resolved context is owned by the graph together with
    /// its concrete extension registry. The executor must use this exact pair
    /// rather than rebuilding model metadata or registry state per phase.
    pub(super) context: SearchContext<S, V, DM, IDM>,
    pub(super) extensions: E,
    pub(super) config: SolverConfig,
    /// Frozen declaration-order bindings for omitted runtime phases. Per-solve
    /// default expansion reads solution state through these slots but never
    /// reconstructs a model/schema or delegates construction to a wrapper.
    pub(super) default_bindings: DefaultRuntimeBindings<S, V, DM, IDM>,
    pub(super) phases: Vec<CompiledRuntimePhase<S, V, DM, IDM>>,
}

impl<S, V, DM, IDM, E> std::fmt::Debug for CompiledRuntimeGraph<S, V, DM, IDM, E>
where
    S: solverforge_core::domain::PlanningSolution,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledRuntimeGraph")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl<S, V, DM, IDM, E> CompiledRuntimeGraph<S, V, DM, IDM, E>
where
    S: solverforge_core::domain::PlanningSolution,
{
    pub(crate) fn context(&self) -> &SearchContext<S, V, DM, IDM> {
        &self.context
    }

    pub(crate) fn config(&self) -> &SolverConfig {
        &self.config
    }

    pub(crate) fn phases(&self) -> &[CompiledRuntimePhase<S, V, DM, IDM>] {
        &self.phases
    }

    pub(crate) fn extensions(&self) -> &E {
        &self.extensions
    }

    pub(crate) fn default_bindings(&self) -> &DefaultRuntimeBindings<S, V, DM, IDM> {
        &self.default_bindings
    }
}
