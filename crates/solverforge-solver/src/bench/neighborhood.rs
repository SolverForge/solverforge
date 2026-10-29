//! The compiled local-search neighborhood and its live step cursor.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::selector::move_selector::{
    CandidateId, MoveStreamContext, ResourceMoveCursor,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::runtime::compiler::executor::local_search::{
    ProviderExecutionResources, RuntimeNeighborhoodCursor, RuntimeNeighborhoodState,
};

use super::CandidateVisitor;

/// One lowered local-search neighborhood with the solve-owned provider
/// resources its cursors borrow.
pub struct CompiledNeighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    state: RuntimeNeighborhoodState<S, V, DM, IDM>,
    resources: ProviderExecutionResources<S>,
}

impl<S, V, DM, IDM> CompiledNeighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    pub(super) fn new(
        state: RuntimeNeighborhoodState<S, V, DM, IDM>,
        resources: ProviderExecutionResources<S>,
    ) -> Self {
        Self { state, resources }
    }

    /// Opens the step cursor a local-search step opens for `context`.
    pub fn open_cursor<D: Director<S>>(
        &mut self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> CompiledNeighborhoodCursor<'_, S, V, DM, IDM> {
        let Self { state, resources } = self;
        let cursor = state.open_cursor_with_resources(resources, score_director, context);
        CompiledNeighborhoodCursor { cursor, resources }
    }
}

impl<S, V, DM, IDM> Debug for CompiledNeighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledNeighborhood")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

/// The live composition cursor of one compiled local-search step. Each pull
/// lends the provider resources exactly as the local-search loop does.
pub struct CompiledNeighborhoodCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    cursor: RuntimeNeighborhoodCursor<'a, S, V, DM, IDM>,
    resources: &'a mut ProviderExecutionResources<S>,
}

impl<S, V, DM, IDM> CompiledNeighborhoodCursor<'_, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    pub fn next_candidate<D: Director<S>>(&mut self, director: &mut D) -> Option<CandidateId> {
        self.cursor
            .next_candidate_with_resources(director, self.resources)
    }

    /// Visits a live candidate; `None` when `id` is not live.
    pub fn visit_candidate<Visitor: CandidateVisitor<S>>(
        &self,
        id: CandidateId,
        visitor: Visitor,
    ) -> Option<Visitor::Output> {
        self.cursor
            .candidate(id)
            .map(|candidate| visitor.visit(candidate))
    }

    pub fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.cursor.release_candidate(id)
    }
}

impl<S, V, DM, IDM> Debug for CompiledNeighborhoodCursor<'_, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledNeighborhoodCursor")
            .finish_non_exhaustive()
    }
}
