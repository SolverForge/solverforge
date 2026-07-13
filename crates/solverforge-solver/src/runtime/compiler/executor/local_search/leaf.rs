//! Frozen compiled local-search leaves and their retained stream state.

use std::fmt::{self, Debug};

use solverforge_config::{SelectionOrder, UnionSelectionOrder};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::builder::context::{ProviderReasonArena, RuntimeProviderRegistry};
use crate::builder::selector::types::StatefulComposedFlat;
use crate::builder::selector::GroupedScalarSelector;
use crate::builder::RuntimeCandidateMetricBinding;
use crate::heuristic::selector::decorator::ResourceVecUnionMoveCursor;
use crate::heuristic::selector::move_selector::{MoveSelector, MoveStreamContext};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::scalar_neighborhood::{
    ScalarNeighborhoodLeaf, ScalarNeighborhoodStreamState,
};
use crate::runtime::compiler::graph::CompiledProviderPlan;
use crate::runtime::provider_cursor::RuntimeProviderCursor;

use super::super::{RuntimeListNeighborhoodLeaf, RuntimeListNeighborhoodStreamState};
use super::cursor::{RuntimeNeighborhoodFlatCursor, RuntimeNeighborhoodLeafCursor};
use super::r#move::RuntimeNeighborhoodMove;

/// The one resource owner for every compiled provider leaf in a solve.
///
/// The runner creates this once from the frozen graph. It survives all
/// sequential local-search phases and pause/resume; leaves and cursors only
/// borrow it at a scheduler-reached pull boundary.
pub(crate) struct ProviderExecutionResources<S>
where
    S: PlanningSolution,
{
    pub(super) registry: RuntimeProviderRegistry<S>,
    pub(super) reason_arena: ProviderReasonArena,
}

impl<S> ProviderExecutionResources<S>
where
    S: PlanningSolution,
{
    pub(crate) fn new(registry: &RuntimeProviderRegistry<S>) -> Self {
        Self {
            registry: registry.clone(),
            reason_arena: ProviderReasonArena::default(),
        }
    }
}

impl<S> Debug for ProviderExecutionResources<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderExecutionResources")
            .field("registry", &self.registry)
            .finish()
    }
}

/// One retained provider declaration. Its cursor is intentionally uninitialized
/// at tree-open time so an unreached child cannot observe registry work.
pub(super) struct RuntimeProviderNeighborhoodLeaf {
    plan: CompiledProviderPlan,
    require_hard_improvement: bool,
}

impl RuntimeProviderNeighborhoodLeaf {
    pub(super) fn new(plan: CompiledProviderPlan, require_hard_improvement: bool) -> Self {
        Self {
            plan,
            require_hard_improvement,
        }
    }

    fn open_cursor<S: PlanningSolution + Clone + 'static, D: Director<S>>(
        &self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> RuntimeProviderNeighborhoodCursor<'_, S> {
        RuntimeProviderNeighborhoodCursor {
            plan: &self.plan,
            solution: Some(score_director.clone_working_solution()),
            context,
            require_hard_improvement: self.require_hard_improvement,
            cursor: None,
        }
    }
}

impl Debug for RuntimeProviderNeighborhoodLeaf {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeProviderNeighborhoodLeaf")
            .field("plan", &self.plan)
            .field("require_hard_improvement", &self.require_hard_improvement)
            .finish()
    }
}

/// One compiled leaf. Scalar/list state is explicit; grouped selectors remain
/// the single retained native selector object, and providers keep no state
/// beyond the runner-owned resource.
#[allow(clippy::large_enum_variant)]
pub(crate) enum RuntimeNeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    Scalar(ScalarNeighborhoodLeaf<S>),
    List(RuntimeListNeighborhoodLeaf<S, V, DM, IDM>),
    Grouped(GroupedScalarSelector<S>),
    Provider(RuntimeProviderNeighborhoodLeaf),
}

pub(crate) enum RuntimeNeighborhoodLeafStreamState {
    Scalar(ScalarNeighborhoodStreamState),
    List(RuntimeListNeighborhoodStreamState),
    Grouped,
    Provider,
}

impl<S, V, DM, IDM> RuntimeNeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    pub(super) fn new_stream_state(&self) -> RuntimeNeighborhoodLeafStreamState {
        match self {
            Self::Scalar(leaf) => {
                RuntimeNeighborhoodLeafStreamState::Scalar(leaf.new_stream_state())
            }
            Self::List(leaf) => RuntimeNeighborhoodLeafStreamState::List(leaf.new_stream_state()),
            Self::Grouped(_) => RuntimeNeighborhoodLeafStreamState::Grouped,
            Self::Provider(_) => RuntimeNeighborhoodLeafStreamState::Provider,
        }
    }

    fn open_cursor_with_stream_state<'a, D: Director<S>>(
        &'a self,
        stream_state: &mut RuntimeNeighborhoodLeafStreamState,
        score_director: &D,
        context: MoveStreamContext,
    ) -> RuntimeNeighborhoodLeafCursor<'a, S, V, DM, IDM> {
        match (self, stream_state) {
            (Self::Scalar(leaf), RuntimeNeighborhoodLeafStreamState::Scalar(state)) => {
                RuntimeNeighborhoodLeafCursor::scalar(leaf.open_cursor_with_stream_state(
                    state,
                    score_director,
                    context,
                ))
            }
            (Self::List(leaf), RuntimeNeighborhoodLeafStreamState::List(state)) => {
                RuntimeNeighborhoodLeafCursor::list(leaf.open_cursor_with_stream_state(
                    state,
                    score_director,
                    context,
                ))
            }
            (Self::Grouped(selector), RuntimeNeighborhoodLeafStreamState::Grouped) => {
                RuntimeNeighborhoodLeafCursor::grouped(
                    selector.open_cursor_with_context(score_director, context),
                )
            }
            (Self::Provider(leaf), RuntimeNeighborhoodLeafStreamState::Provider) => {
                RuntimeNeighborhoodLeafCursor::provider(leaf.open_cursor(score_director, context))
            }
            _ => panic!("compiled runtime neighborhood stream state must match its leaf"),
        }
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Scalar(leaf) => MoveSelector::size(leaf, score_director),
            Self::List(leaf) => leaf.size(score_director),
            Self::Grouped(selector) => MoveSelector::size(selector, score_director),
            Self::Provider(_) => 0,
        }
    }
}

impl<S, V, DM, IDM> Debug for RuntimeNeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(leaf) => formatter
                .debug_tuple("RuntimeNeighborhoodLeaf::Scalar")
                .field(leaf)
                .finish(),
            Self::List(leaf) => formatter
                .debug_tuple("RuntimeNeighborhoodLeaf::List")
                .field(leaf)
                .finish(),
            Self::Grouped(selector) => formatter
                .debug_tuple("RuntimeNeighborhoodLeaf::Grouped")
                .field(selector)
                .finish(),
            Self::Provider(leaf) => formatter
                .debug_tuple("RuntimeNeighborhoodLeaf::Provider")
                .field(leaf)
                .finish(),
        }
    }
}

/// A frozen flat declaration-order leaf set. Its one resource-aware union
/// cursor supplies the same canonical sequential/round-robin scheduler used
/// by ordinary selector unions; no compiled-specific scheduler exists.
pub(crate) struct RuntimeNeighborhoodFlat<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    leaves: Vec<RuntimeNeighborhoodLeaf<S, V, DM, IDM>>,
    selection_order: UnionSelectionOrder,
    candidate_order: SelectionOrder,
    candidate_metric: Option<RuntimeCandidateMetricBinding<S>>,
}

impl<S, V, DM, IDM> RuntimeNeighborhoodFlat<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    pub(super) fn new(
        leaves: Vec<RuntimeNeighborhoodLeaf<S, V, DM, IDM>>,
        selection_order: UnionSelectionOrder,
        candidate_order: SelectionOrder,
        candidate_metric: Option<RuntimeCandidateMetricBinding<S>>,
    ) -> Self {
        assert!(
            !leaves.is_empty(),
            "compiled neighborhood flat must retain at least one frozen leaf"
        );
        Self {
            leaves,
            selection_order,
            candidate_order,
            candidate_metric,
        }
    }
}

impl<S, V, DM, IDM> Debug for RuntimeNeighborhoodFlat<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeNeighborhoodFlat")
            .field("leaves", &self.leaves)
            .field("selection_order", &self.selection_order)
            .field("candidate_order", &self.candidate_order)
            .field(
                "candidate_metric",
                &self.candidate_metric.as_ref().map(|metric| metric.name()),
            )
            .finish()
    }
}

impl<S, V, DM, IDM>
    StatefulComposedFlat<
        S,
        RuntimeNeighborhoodMove<S, V, DM, IDM>,
        Vec<RuntimeNeighborhoodLeafStreamState>,
        ProviderExecutionResources<S>,
    > for RuntimeNeighborhoodFlat<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    type Cursor<'a>
        = RuntimeNeighborhoodFlatCursor<'a, S, V, DM, IDM>
    where
        Self: 'a;

    fn new_stream_state(&self) -> Vec<RuntimeNeighborhoodLeafStreamState> {
        self.leaves
            .iter()
            .map(RuntimeNeighborhoodLeaf::new_stream_state)
            .collect()
    }

    fn open_cursor_with_stream_state<'a, D: Director<S>>(
        &'a self,
        stream_state: &mut Vec<RuntimeNeighborhoodLeafStreamState>,
        _resources: &mut ProviderExecutionResources<S>,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        assert_eq!(
            stream_state.len(),
            self.leaves.len(),
            "compiled neighborhood state must match frozen leaves"
        );
        let candidate_context = context.with_selection_order(self.candidate_order);
        let cursors = self
            .leaves
            .iter()
            .zip(stream_state.iter_mut())
            .map(|(leaf, state)| {
                leaf.open_cursor_with_stream_state(state, score_director, candidate_context)
            })
            .collect();
        RuntimeNeighborhoodFlatCursor::new(
            ResourceVecUnionMoveCursor::new(
                cursors,
                self.selection_order,
                context,
                vec![1; self.leaves.len()],
            ),
            self.candidate_order,
            self.candidate_metric.clone(),
            self.candidate_order
                .requires_complete_stream()
                .then(|| score_director.clone_working_solution()),
            candidate_context,
        )
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.leaves
            .iter()
            .map(|leaf| leaf.size(score_director))
            .sum()
    }

    fn validate_cursor<D: Director<S>>(&self, _score_director: &D) {
        // Compiler validation has already frozen exact capabilities. Opening
        // a compiled cursor performs no separate selector validation pass.
    }
}

/// Lazy provider child cursor. It captures only an immutable plan reference
/// and one solution snapshot at tree-open time, then creates the provider
/// cursor only when the scheduler reaches this child.
pub(super) struct RuntimeProviderNeighborhoodCursor<'a, S>
where
    S: PlanningSolution + Clone + 'static,
{
    plan: &'a CompiledProviderPlan,
    solution: Option<S>,
    context: MoveStreamContext,
    require_hard_improvement: bool,
    cursor: Option<RuntimeProviderCursor<S>>,
}

impl<S> RuntimeProviderNeighborhoodCursor<'_, S>
where
    S: PlanningSolution + Clone + 'static,
{
    pub(super) fn next_candidate(
        &mut self,
        resources: &mut ProviderExecutionResources<S>,
    ) -> Option<crate::heuristic::selector::move_selector::CandidateId> {
        let cursor = self.cursor.get_or_insert_with(|| {
            RuntimeProviderCursor::new(
                self.plan.clone(),
                self.solution
                    .take()
                    .expect("provider solution snapshot must be retained until activation"),
                self.context,
                self.require_hard_improvement,
            )
        });
        cursor.next_candidate(&resources.registry, &mut resources.reason_arena)
    }

    pub(super) fn cursor_mut(&mut self) -> &mut RuntimeProviderCursor<S> {
        self.cursor
            .as_mut()
            .expect("provider candidate access requires an activated provider cursor")
    }
}
