use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, LocalSearchConfig, MoveSelectorConfig,
    PartitionedSearchConfig, SelectionOrder, SolverConfig, UnionSelectionOrder, UnionWeighting,
};

use crate::builder::{
    CompiledAcceptor, CompiledForager, RuntimeCandidateMetricBinding, RuntimeProviderHandle,
    RuntimeScalarSlotId, ScalarGroupBinding, SearchContext,
};
use crate::descriptor::ResolvedVariableBinding;
use crate::phase::construction::{ScalarConstructionSchedule, ScalarOrMixedSlotOrder};

use super::defaults::DefaultRuntimeBindings;
use super::types::{CompiledListSlot, CompiledScalarSlot};

mod provider;
pub(crate) use provider::{
    CompiledProviderPlan, ProviderBindingPlan, ProviderBindingPolicy, ProviderMoveKind,
    ProviderPullTiming, ProviderSchedule,
};
#[cfg(test)]
pub(crate) use provider::{
    ProviderCandidateContract, ProviderCandidateDeduplication, ProviderReasonStorage,
    ProviderTabuIdentity,
};
pub(super) use provider::{
    GROUP_PROVIDER_ROTATION_SALT, PYTHON_PROVIDER_CANDIDATE_CONTRACT,
    REPAIR_PROVIDER_ROTATION_SALT, STATIC_GROUP_PROVIDER_ROTATION_SALT,
    STATIC_REPAIR_CONSTRAINT_ROTATION_SALT, STATIC_REPAIR_PROVIDER_ROTATION_SALT,
    STATIC_REPAIR_SPEC_ROTATION_SALT,
};

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

/// Stable coordinate into one immutable compiled selector declaration arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CompiledSelectorNodeId(usize);

impl CompiledSelectorNodeId {
    pub(super) fn new(index: usize) -> Self {
        Self(index)
    }

    pub(super) fn index(self) -> usize {
        self.0
    }
}

/// One immutable selector declaration node. Structural edges use stable arena
/// coordinates, so nested selectors never require recursive heap ownership.
#[derive(Clone, Debug)]
#[allow(
    clippy::large_enum_variant,
    reason = "leaf declarations carry their frozen slot vectors"
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
        selector: CompiledSelectorNodeId,
    },
    Union {
        selection_order: UnionSelectionOrder,
        weighting: UnionWeighting,
        weights: Vec<u64>,
        children: Vec<CompiledSelectorNodeId>,
    },
    Cartesian {
        require_hard_improvement: bool,
        left: CompiledSelectorNodeId,
        right: CompiledSelectorNodeId,
    },
}

/// One postorder selector graph with a stable root coordinate.
#[derive(Clone, Debug)]
pub(crate) struct CompiledSelectorGraph<S, V, DM, IDM> {
    nodes: Vec<CompiledSelectorNode<S, V, DM, IDM>>,
    root: CompiledSelectorNodeId,
}

impl<S, V, DM, IDM> CompiledSelectorGraph<S, V, DM, IDM> {
    pub(super) fn from_nodes_and_root(
        nodes: Vec<CompiledSelectorNode<S, V, DM, IDM>>,
        root: CompiledSelectorNodeId,
    ) -> Self {
        assert!(
            root.index() == nodes.len() - 1,
            "compiled selector graph root must be the last pushed node"
        );
        Self { nodes, root }
    }

    /// Consumes the arena; each node lowers exactly once per solve.
    pub(super) fn into_parts(
        self,
    ) -> (
        Vec<Option<CompiledSelectorNode<S, V, DM, IDM>>>,
        CompiledSelectorNodeId,
    ) {
        (self.nodes.into_iter().map(Some).collect(), self.root)
    }

    pub(super) fn node(&self, id: CompiledSelectorNodeId) -> &CompiledSelectorNode<S, V, DM, IDM> {
        &self.nodes[id.index()]
    }

    pub(super) fn requires_score_during_move_root(&self) -> bool {
        matches!(
            self.node(self.root),
            CompiledSelectorNode::Scalar {
                kind: ScalarLeafKind::RuinRecreate,
                ..
            } | CompiledSelectorNode::List {
                kind: ListLeafKind::Ruin,
                ..
            }
        )
    }

    pub(super) fn contains_provider_root(&self) -> bool {
        matches!(self.node(self.root), CompiledSelectorNode::Provider { .. })
    }
}

pub(super) struct SelectorGraphBuilder<S, V, DM, IDM> {
    nodes: Vec<CompiledSelectorNode<S, V, DM, IDM>>,
}

impl<S, V, DM, IDM> Default for SelectorGraphBuilder<S, V, DM, IDM> {
    fn default() -> Self {
        Self { nodes: Vec::new() }
    }
}

impl<S, V, DM, IDM> SelectorGraphBuilder<S, V, DM, IDM> {
    pub(super) fn push(
        &mut self,
        node: CompiledSelectorNode<S, V, DM, IDM>,
    ) -> CompiledSelectorNodeId {
        let id = CompiledSelectorNodeId::new(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub(super) fn assemble_graph(
        self,
        root: CompiledSelectorNodeId,
    ) -> CompiledSelectorGraph<S, V, DM, IDM> {
        assert!(
            root.index() == self.nodes.len() - 1,
            "compiled selector graph root must be the last pushed node"
        );
        CompiledSelectorGraph::from_nodes_and_root(self.nodes, root)
    }
}

/// Merges one compiled sub-graph's nodes into `builder` in order and returns
/// the rebased root id. Node contents stay untouched; every structural edge —
/// child IDs stored inside node contents as well as the root — is shifted by the
/// merged offset, because each sub-graph carries its own zero-based arena
/// coordinates.
pub(super) fn rebase_graph<S, V, DM, IDM>(
    builder: &mut SelectorGraphBuilder<S, V, DM, IDM>,
    graph: CompiledSelectorGraph<S, V, DM, IDM>,
) -> CompiledSelectorNodeId {
    let offset = builder.nodes.len();
    for node in graph.nodes {
        let rebased = match node {
            CompiledSelectorNode::Limited {
                selected_count_limit,
                selector,
            } => CompiledSelectorNode::Limited {
                selected_count_limit,
                selector: CompiledSelectorNodeId::new(selector.index() + offset),
            },
            CompiledSelectorNode::Union {
                selection_order,
                weighting,
                weights,
                children,
            } => CompiledSelectorNode::Union {
                selection_order,
                weighting,
                weights,
                children: children
                    .into_iter()
                    .map(|child| CompiledSelectorNodeId::new(child.index() + offset))
                    .collect(),
            },
            CompiledSelectorNode::Cartesian {
                require_hard_improvement,
                left,
                right,
            } => CompiledSelectorNode::Cartesian {
                require_hard_improvement,
                left: CompiledSelectorNodeId::new(left.index() + offset),
                right: CompiledSelectorNodeId::new(right.index() + offset),
            },
            leaf => leaf,
        };
        builder.nodes.push(rebased);
    }
    CompiledSelectorNodeId::new(graph.root.index() + offset)
}

#[derive(Clone, Debug)]
#[allow(
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
pub(crate) enum CompiledAcceptorForagerSelector<S, V, DM, IDM> {
    /// A user-declared selector compiled directly into the immutable graph.
    Explicit(CompiledSelectorGraph<S, V, DM, IDM>),
    /// This phase intentionally consumes `DefaultRuntimeBindings`' one
    /// capability-resolved declaration. It is not an absent selector or a
    /// late-selected alternative.
    OmittedDefault,
}

#[derive(Clone, Debug)]
pub(crate) enum CompiledLocalSearch<S, V, DM, IDM> {
    AcceptorForager {
        config: LocalSearchConfig,
        /// The configured acceptor, validated for this score type.
        acceptor: Option<CompiledAcceptor<S>>,
        /// The configured forager, validated.
        forager: Option<CompiledForager>,
        selector: CompiledAcceptorForagerSelector<S, V, DM, IDM>,
    },
    VariableNeighborhoodDescent {
        config: LocalSearchConfig,
        neighborhoods: Vec<CompiledSelectorGraph<S, V, DM, IDM>>,
    },
}

#[derive(Clone, Debug)]
#[allow(
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
