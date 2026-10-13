//! Stable union coordinates backed by the canonical scheduling algorithm.
use super::CompositionNodeId;
use crate::heuristic::selector::decorator::UnionScheduler;
use crate::heuristic::selector::move_selector::{CandidateId, MoveStreamContext};
use solverforge_config::UnionSelectionOrder;

pub(super) struct SelectorCompositionUnionCursor {
    scheduler: UnionScheduler,
    discovered: Vec<(usize, CompositionNodeId, CandidateId)>,
}
impl SelectorCompositionUnionCursor {
    pub(super) fn new(
        children: &[CompositionNodeId],
        order: UnionSelectionOrder,
        context: MoveStreamContext,
        weights: Vec<u64>,
    ) -> Self {
        Self {
            scheduler: UnionScheduler::new(children.len(), order, context, weights),
            discovered: Vec::new(),
        }
    }
    pub(super) fn next(
        &mut self,
        children: &[CompositionNodeId],
        mut pull: impl FnMut(CompositionNodeId) -> Option<CandidateId>,
    ) -> Option<CandidateId> {
        let (index, id) = self
            .scheduler
            .next_child(children.len(), |index| pull(children[index]))?;
        let global = CandidateId::new(self.discovered.len());
        self.discovered.push((index, children[index], id));
        Some(global)
    }
    pub(super) fn coordinate(&self, id: CandidateId) -> Option<(CompositionNodeId, CandidateId)> {
        let &(_, node, child) = self.discovered.get(id.index())?;
        Some((node, child))
    }
    pub(super) fn selector_index(&self, id: CandidateId) -> Option<usize> {
        self.discovered.get(id.index()).map(|&(index, _, _)| index)
    }
}
