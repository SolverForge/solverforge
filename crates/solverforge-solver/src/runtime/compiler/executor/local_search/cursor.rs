//! Resource-aware cursors for the compiled local-search carrier.

use std::fmt::Debug;

use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use solverforge_config::SelectionOrder;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::builder::selector::GroupedScalarCursor;
use crate::builder::RuntimeCandidateMetricBinding;
use crate::heuristic::r#move::Move;
use crate::heuristic::selector::decorator::ResourceVecUnionMoveCursor;
use crate::heuristic::selector::move_selector::MoveStreamContext;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, ResourceMoveCursor,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::scalar_neighborhood::RuntimeScalarNeighborhoodCursor;

use super::super::RuntimeListNeighborhoodCursor;
use super::leaf::{ProviderExecutionResources, RuntimeProviderNeighborhoodCursor};
use super::r#move::RuntimeNeighborhoodMove;

#[allow(clippy::large_enum_variant)]
enum RuntimeNeighborhoodLeafCursorInner<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    Scalar(RuntimeScalarNeighborhoodCursor<S>),
    List(RuntimeListNeighborhoodCursor<'a, S, V, DM, IDM>),
    Grouped(GroupedScalarCursor<S>),
    Provider(RuntimeProviderNeighborhoodCursor<'a, S>),
}

/// Maps one concrete frozen leaf into the one runtime carrier while retaining
/// candidate ownership in a cursor-local store. It is not a second selector:
/// the flat union owns scheduling and lends provider resources exactly here.
pub(crate) struct RuntimeNeighborhoodLeafCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    inner: RuntimeNeighborhoodLeafCursorInner<'a, S, V, DM, IDM>,
    store: CandidateStore<S, RuntimeNeighborhoodMove<S, V, DM, IDM>>,
}

impl<'a, S, V, DM, IDM> RuntimeNeighborhoodLeafCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    pub(super) fn scalar(cursor: RuntimeScalarNeighborhoodCursor<S>) -> Self {
        Self {
            inner: RuntimeNeighborhoodLeafCursorInner::Scalar(cursor),
            store: CandidateStore::new(),
        }
    }

    pub(super) fn list(cursor: RuntimeListNeighborhoodCursor<'a, S, V, DM, IDM>) -> Self {
        Self {
            inner: RuntimeNeighborhoodLeafCursorInner::List(cursor),
            store: CandidateStore::new(),
        }
    }

    pub(super) fn grouped(cursor: GroupedScalarCursor<S>) -> Self {
        Self {
            inner: RuntimeNeighborhoodLeafCursorInner::Grouped(cursor),
            store: CandidateStore::new(),
        }
    }

    pub(super) fn provider(cursor: RuntimeProviderNeighborhoodCursor<'a, S>) -> Self {
        Self {
            inner: RuntimeNeighborhoodLeafCursorInner::Provider(cursor),
            store: CandidateStore::new(),
        }
    }

    fn next_move(
        &mut self,
        resources: &mut ProviderExecutionResources<S>,
    ) -> Option<RuntimeNeighborhoodMove<S, V, DM, IDM>> {
        match &mut self.inner {
            RuntimeNeighborhoodLeafCursorInner::Scalar(cursor) => cursor
                .next_owned_candidate()
                .map(RuntimeNeighborhoodMove::Scalar),
            RuntimeNeighborhoodLeafCursorInner::List(cursor) => cursor
                .next_owned_candidate()
                .map(RuntimeNeighborhoodMove::List),
            RuntimeNeighborhoodLeafCursorInner::Grouped(cursor) => cursor
                .next_owned_candidate()
                .map(RuntimeNeighborhoodMove::Grouped),
            RuntimeNeighborhoodLeafCursorInner::Provider(cursor) => {
                let candidate = cursor.next_candidate(resources)?;
                Some(RuntimeNeighborhoodMove::Provider(
                    cursor.cursor_mut().take_candidate(candidate),
                ))
            }
        }
    }
}

impl<S, V, DM, IDM>
    ResourceMoveCursor<S, RuntimeNeighborhoodMove<S, V, DM, IDM>, ProviderExecutionResources<S>>
    for RuntimeNeighborhoodLeafCursor<'_, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    fn next_candidate_with_resources(
        &mut self,
        resources: &mut ProviderExecutionResources<S>,
    ) -> Option<CandidateId> {
        self.next_move(resources).map(|mov| self.store.push(mov))
    }

    fn candidate(
        &self,
        index: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, RuntimeNeighborhoodMove<S, V, DM, IDM>>> {
        self.store.candidate(index)
    }

    fn take_candidate(&mut self, index: CandidateId) -> RuntimeNeighborhoodMove<S, V, DM, IDM> {
        self.store.take_candidate(index)
    }

    fn apply_owned_candidate<D: Director<S>>(
        &mut self,
        index: CandidateId,
        score_director: &mut D,
    ) {
        self.take_candidate(index).do_move(score_director);
    }

    fn release_candidate(&mut self, index: CandidateId) -> bool {
        self.store.release_candidate(index)
    }

    fn selector_index(&self, _index: CandidateId) -> Option<usize> {
        None
    }
}

/// The one canonical resource-aware union over a compiled flat leaf set.
pub(crate) struct RuntimeNeighborhoodFlatCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    inner: ResourceVecUnionMoveCursor<
        S,
        RuntimeNeighborhoodMove<S, V, DM, IDM>,
        RuntimeNeighborhoodLeafCursor<'a, S, V, DM, IDM>,
    >,
    candidate_order: SelectionOrder,
    candidate_metric: Option<RuntimeCandidateMetricBinding<S>>,
    metric_solution: Option<S>,
    context: MoveStreamContext,
    ordered_candidates: Option<Vec<CandidateId>>,
    ordered_offset: usize,
}

impl<'a, S, V, DM, IDM> RuntimeNeighborhoodFlatCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    pub(super) fn new(
        inner: ResourceVecUnionMoveCursor<
            S,
            RuntimeNeighborhoodMove<S, V, DM, IDM>,
            RuntimeNeighborhoodLeafCursor<'a, S, V, DM, IDM>,
        >,
        candidate_order: SelectionOrder,
        candidate_metric: Option<RuntimeCandidateMetricBinding<S>>,
        metric_solution: Option<S>,
        context: MoveStreamContext,
    ) -> Self {
        Self {
            inner,
            candidate_order,
            candidate_metric,
            metric_solution,
            context,
            ordered_candidates: None,
            ordered_offset: 0,
        }
    }

    fn prepare_order(&mut self, resources: &mut ProviderExecutionResources<S>) {
        if self.ordered_candidates.is_some() {
            return;
        }
        let metric = self
            .candidate_metric
            .as_ref()
            .expect("sorted and probabilistic selectors must retain their compiled metric");
        let solution = self
            .metric_solution
            .as_ref()
            .expect("metric-backed selector must retain its cursor-open solution snapshot");
        let mut candidates = Vec::new();
        while let Some(candidate_id) = self.inner.next_candidate_with_resources(resources) {
            let candidate = self
                .inner
                .candidate(candidate_id)
                .expect("discovered metric candidate must remain valid");
            let identity = candidate
                .candidate_trace_identity()
                .expect("compiled runtime candidate must expose a logical identity");
            let value = metric.measure(solution, &identity);
            assert!(
                value.is_finite(),
                "candidate metric `{}` returned a non-finite value",
                metric.name()
            );
            candidates.push((candidate_id, value));
        }

        match self.candidate_order {
            SelectionOrder::Sorted => {
                candidates.sort_by(|(_, left), (_, right)| left.total_cmp(right));
            }
            SelectionOrder::Probabilistic => {
                let mut rng =
                    StdRng::seed_from_u64(self.context.random_seed(0xA17E_93C4_D58B_6201));
                let mut weighted = Vec::with_capacity(candidates.len());
                for (candidate_id, weight) in candidates {
                    assert!(
                        weight >= 0.0,
                        "candidate metric `{}` returned a negative probabilistic weight",
                        metric.name()
                    );
                    if weight == 0.0 {
                        assert!(self.inner.release_candidate(candidate_id));
                        continue;
                    }
                    let draw = rng.random::<f64>().max(f64::MIN_POSITIVE);
                    weighted.push((candidate_id, -draw.ln() / weight));
                }
                weighted.sort_by(|(_, left), (_, right)| left.total_cmp(right));
                candidates = weighted;
            }
            SelectionOrder::Original | SelectionOrder::Random | SelectionOrder::Shuffled => {
                unreachable!("only metric-backed candidate orders materialize a stream")
            }
        }
        self.ordered_candidates = Some(
            candidates
                .into_iter()
                .map(|(candidate_id, _)| candidate_id)
                .collect(),
        );
    }
}

impl<S, V, DM, IDM>
    ResourceMoveCursor<S, RuntimeNeighborhoodMove<S, V, DM, IDM>, ProviderExecutionResources<S>>
    for RuntimeNeighborhoodFlatCursor<'_, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    fn next_candidate_with_resources(
        &mut self,
        resources: &mut ProviderExecutionResources<S>,
    ) -> Option<CandidateId> {
        if self.candidate_order.requires_complete_stream() {
            self.prepare_order(resources);
            let candidate = self
                .ordered_candidates
                .as_ref()
                .and_then(|candidates| candidates.get(self.ordered_offset))
                .copied();
            self.ordered_offset += usize::from(candidate.is_some());
            candidate
        } else {
            self.inner.next_candidate_with_resources(resources)
        }
    }

    fn candidate(
        &self,
        index: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, RuntimeNeighborhoodMove<S, V, DM, IDM>>> {
        self.inner.candidate(index)
    }

    fn take_candidate(&mut self, index: CandidateId) -> RuntimeNeighborhoodMove<S, V, DM, IDM> {
        self.inner.take_candidate(index)
    }

    fn apply_owned_candidate<D: Director<S>>(
        &mut self,
        index: CandidateId,
        score_director: &mut D,
    ) {
        self.inner.apply_owned_candidate(index, score_director);
    }

    fn release_candidate(&mut self, index: CandidateId) -> bool {
        self.inner.release_candidate(index)
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        self.inner.selector_index(index)
    }
}
