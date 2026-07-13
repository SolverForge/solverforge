//! Leaf-only lowering from one immutable compiled list declaration.

use std::fmt;

use solverforge_config::SolverConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::runtime::compiler::graph::CompiledSelectorNode;

use super::{
    RuntimeListNeighborhoodCursor, RuntimeListNeighborhoodPlan, RuntimeListNeighborhoodPlanError,
    RuntimeListNeighborhoodSelector, RuntimeListNeighborhoodStreamState,
};

/// One compiled list leaf for the generic local-search composer.
pub(crate) struct RuntimeListNeighborhoodLeaf<S, V, DM, IDM> {
    selector: RuntimeListNeighborhoodSelector<S, V, DM, IDM>,
}

impl<S, V, DM, IDM> RuntimeListNeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn new(plan: RuntimeListNeighborhoodPlan<S, V, DM, IDM>) -> Self {
        Self {
            selector: RuntimeListNeighborhoodSelector::new(plan),
        }
    }

    pub(crate) fn plan(&self) -> &RuntimeListNeighborhoodPlan<S, V, DM, IDM> {
        self.selector.plan()
    }

    /// Creates the leaf's mutable per-solve stream state. The compiled leaf
    /// remains immutable across retained solves and can never own a hidden
    /// RNG behind interior mutability.
    pub(crate) fn new_stream_state(&self) -> RuntimeListNeighborhoodStreamState {
        self.selector.new_stream_state()
    }

    /// Opens through the generic compositor's state-owning boundary.
    ///
    /// The cursor owns all derived ruin seeds and candidate recipes, so the
    /// mutable state borrow ends before the cursor is returned.
    pub(crate) fn open_cursor_with_stream_state<'a, D: Director<S>>(
        &'a self,
        stream_state: &mut RuntimeListNeighborhoodStreamState,
        score_director: &D,
        context: crate::heuristic::selector::move_selector::MoveStreamContext,
    ) -> RuntimeListNeighborhoodCursor<'a, S, V, DM, IDM> {
        self.selector
            .open_cursor_with_stream_state(stream_state, score_director, context)
    }

    pub(crate) fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        crate::heuristic::selector::move_selector::MoveSelector::size(
            &self.selector,
            score_director,
        )
    }
}

impl<S, V, DM, IDM> fmt::Debug for RuntimeListNeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeListNeighborhoodLeaf")
            .field("kind", &self.plan().kind())
            .field("slot_count", &self.plan().slots().len())
            .finish_non_exhaustive()
    }
}

/// Solve-independent adapter from one frozen compiler node.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CompiledListNeighborhoodLeafAdapter {
    random_seed: Option<u64>,
}

impl CompiledListNeighborhoodLeafAdapter {
    pub(crate) fn from_solver_config(config: &SolverConfig) -> Self {
        Self {
            random_seed: config.random_seed,
        }
    }

    /// Lowers exactly one already-compiled list leaf. Recursive composition
    /// remains above this boundary and no alternate native selector is built.
    pub(crate) fn lower_compiled_node<S, V, DM, IDM>(
        self,
        node: &CompiledSelectorNode<S, V, DM, IDM>,
    ) -> Result<RuntimeListNeighborhoodLeaf<S, V, DM, IDM>, RuntimeListNeighborhoodLeafError>
    where
        S: PlanningSolution + Clone + Send + Sync + 'static,
        S::Score: Score,
        V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
        DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
        IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    {
        let CompiledSelectorNode::List {
            kind,
            config,
            slots,
            ..
        } = node
        else {
            return Err(RuntimeListNeighborhoodLeafError::NotListNode);
        };
        RuntimeListNeighborhoodPlan::from_compiled(*kind, config, slots.clone(), self.random_seed)
            .map(RuntimeListNeighborhoodLeaf::new)
            .map_err(RuntimeListNeighborhoodLeafError::Plan)
    }
}

/// Invalid compiled-leaf handoff. These errors never trigger a fallback.
#[derive(Debug)]
pub(crate) enum RuntimeListNeighborhoodLeafError {
    NotListNode,
    Plan(RuntimeListNeighborhoodPlanError),
}

impl fmt::Display for RuntimeListNeighborhoodLeafError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotListNode => {
                formatter.write_str("compiled local-search node is not a list leaf")
            }
            Self::Plan(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for RuntimeListNeighborhoodLeafError {}
