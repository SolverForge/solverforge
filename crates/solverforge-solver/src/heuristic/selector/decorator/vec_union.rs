/* Canonical vector-backed union scheduling for selector composition. */

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_config::{SelectionOrder, UnionSelectionOrder, UnionWeighting};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext, ResourceMoveCursor,
};

/// Combines moves from an arbitrary number of leaf selectors into a single stream.
pub struct VecUnionSelector<S, M, Leaf> {
    selectors: Vec<Leaf>,
    selection_order: UnionSelectionOrder,
    weighting: UnionWeighting,
    weights: Vec<u64>,
    candidate_order: SelectionOrder,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, Leaf> VecUnionSelector<S, M, Leaf> {
    pub fn new(selectors: Vec<Leaf>) -> Self {
        Self::with_selection_order(selectors, UnionSelectionOrder::default())
    }

    pub fn with_selection_order(
        selectors: Vec<Leaf>,
        selection_order: UnionSelectionOrder,
    ) -> Self {
        Self {
            selectors,
            selection_order,
            weighting: UnionWeighting::Equal,
            weights: Vec::new(),
            candidate_order: SelectionOrder::Original,
            _phantom: PhantomData,
        }
    }

    pub fn with_weighting(mut self, weighting: UnionWeighting, weights: Vec<u64>) -> Self {
        self.weighting = weighting;
        self.weights = weights;
        self
    }

    pub fn with_candidate_order(mut self, candidate_order: SelectionOrder) -> Self {
        self.candidate_order = candidate_order;
        self
    }

    pub fn selectors(&self) -> &[Leaf] {
        &self.selectors
    }

    pub fn into_selectors(self) -> Vec<Leaf> {
        self.selectors
    }

    pub fn selection_order(&self) -> UnionSelectionOrder {
        self.selection_order
    }

    pub fn weighting(&self) -> UnionWeighting {
        self.weighting
    }

    pub fn weights(&self) -> &[u64] {
        &self.weights
    }

    pub fn candidate_order(&self) -> SelectionOrder {
        self.candidate_order
    }

    pub(crate) fn resolved_weights(&self, child_sizes: &[usize]) -> Vec<u64> {
        resolve_union_weights(self.weighting, &self.weights, child_sizes)
    }

    /// Applies the same child-context rule used by the canonical union cursor.
    ///
    /// Stateful composition owns leaf stream state outside this selector, but
    /// must retain the exact union seeding and ordering contract when it opens
    /// those leaves.  Keeping the rule here prevents a parallel union path.
    pub(crate) fn child_context(&self, context: MoveStreamContext) -> MoveStreamContext {
        union_child_context(self.selection_order, context)
            .with_selection_order(self.candidate_order)
    }

    /// Builds the canonical union cursor from already-opened leaf cursors.
    ///
    /// This is intentionally the only escape hatch needed by the composed
    /// execution state tree: ownership, candidate IDs, release, seed-derived
    /// rotation, and lazy pull ordering still live in `VecUnionMoveCursor`.
    pub(crate) fn cursor_from_opened<C>(
        &self,
        cursors: Vec<C>,
        context: MoveStreamContext,
        weights: Vec<u64>,
    ) -> VecUnionMoveCursor<S, M, C>
    where
        S: PlanningSolution,
        M: Move<S>,
        C: MoveCursor<S, M>,
    {
        union_cursor_from_opened(cursors, self.selection_order, context, weights)
    }
}

/// Applies the canonical union child-context rule without retaining a frozen
/// `VecUnionSelector` declaration.
///
/// Stateful composition executions consume that declaration so they can own
/// one stream-state tree for the whole solve. They call this helper instead
/// of recreating union seeding or ordering behavior at the phase boundary.
pub(crate) fn union_child_context(
    _selection_order: UnionSelectionOrder,
    context: MoveStreamContext,
) -> MoveStreamContext {
    context
}

/// Builds the one canonical union cursor from already-opened child cursors.
///
/// This is deliberately shared by `VecUnionSelector` and composition
/// executions. Candidate IDs, release, selection order, and seed-derived
/// rotation therefore remain implemented in `VecUnionMoveCursor` exactly
/// once.
pub(crate) fn union_cursor_from_opened<S, M, C>(
    cursors: Vec<C>,
    selection_order: UnionSelectionOrder,
    context: MoveStreamContext,
    weights: Vec<u64>,
) -> VecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    VecUnionMoveCursor::new(cursors, selection_order, context, weights)
}

pub(crate) fn resolve_union_weights(
    weighting: UnionWeighting,
    fixed: &[u64],
    child_sizes: &[usize],
) -> Vec<u64> {
    match weighting {
        UnionWeighting::Equal => {
            assert!(
                fixed.is_empty(),
                "equal union weighting does not accept explicit weights"
            );
            vec![1; child_sizes.len()]
        }
        UnionWeighting::Fixed => {
            assert_eq!(
                fixed.len(),
                child_sizes.len(),
                "fixed union weight count must match child count"
            );
            assert!(
                fixed.iter().all(|weight| *weight > 0),
                "fixed union weights must all be positive"
            );
            fixed.to_vec()
        }
        UnionWeighting::CandidateCount => {
            assert!(
                fixed.is_empty(),
                "candidate_count union weighting does not accept explicit weights"
            );
            child_sizes
                .iter()
                .map(|size| u64::try_from(*size).unwrap_or(u64::MAX))
                .collect()
        }
    }
}

mod scheduler;
pub(crate) use scheduler::UnionScheduler;

impl<S, M, Leaf: Debug> Debug for VecUnionSelector<S, M, Leaf> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VecUnionSelector")
            .field("selectors", &self.selectors)
            .field("selection_order", &self.selection_order)
            .field("weighting", &self.weighting)
            .field("weights", &self.weights)
            .finish()
    }
}

impl<S, M, Leaf> MoveSelector<S, M> for VecUnionSelector<S, M, Leaf>
where
    S: PlanningSolution,
    M: Move<S>,
    Leaf: MoveSelector<S, M>,
{
    type Cursor<'a>
        = VecUnionMoveCursor<S, M, Leaf::Cursor<'a>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        let child_sizes = self
            .selectors
            .iter()
            .map(|selector| selector.size(score_director))
            .collect::<Vec<_>>();
        let child_context = self.child_context(context);
        let cursors = self
            .selectors
            .iter()
            .map(|selector| selector.open_cursor_with_context(score_director, child_context))
            .collect();
        self.cursor_from_opened(cursors, context, self.resolved_weights(&child_sizes))
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.selectors.iter().map(|s| s.size(score_director)).sum()
    }

    fn validate_cursor<D: Director<S>>(&self, score_director: &D) {
        for selector in &self.selectors {
            selector.validate_cursor(score_director);
        }
    }
}

pub struct VecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    cursors: Vec<C>,
    scheduler: UnionScheduler,
    discovered: Vec<(usize, CandidateId)>,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, C> VecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    pub(crate) fn new(
        cursors: Vec<C>,
        selection_order: UnionSelectionOrder,
        context: MoveStreamContext,
        weights: Vec<u64>,
    ) -> Self {
        Self {
            scheduler: UnionScheduler::new(cursors.len(), selection_order, context, weights),
            cursors,
            discovered: Vec::new(),
            _phantom: PhantomData,
        }
    }

    fn push_discovered(&mut self, cursor_index: usize, child_index: CandidateId) -> CandidateId {
        let global_id = CandidateId::new(self.discovered.len());
        self.discovered.push((cursor_index, child_index));
        self.cursors[cursor_index]
            .candidate(child_index)
            .expect("vec union candidate must remain valid");
        global_id
    }
}

impl<S, M, C> MoveCursor<S, M> for VecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn next_candidate(&mut self, solution: &S) -> Option<CandidateId> {
        let cursor_count = self.cursors.len();
        let (cursor_index, child_index) = self.scheduler.next_child(cursor_count, |index| {
            self.cursors[index].next_candidate(solution)
        })?;
        Some(self.push_discovered(cursor_index, child_index))
    }

    fn next_candidate_with_director<D: Director<S>>(
        &mut self,
        director: &mut D,
    ) -> Option<CandidateId> {
        let cursor_count = self.cursors.len();
        let (cursor_index, child_index) = self.scheduler.next_child(cursor_count, |index| {
            self.cursors[index].next_candidate_with_director(director)
        })?;
        Some(self.push_discovered(cursor_index, child_index))
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        let (cursor_index, child_index) = *self.discovered.get(index.index())?;
        self.cursors[cursor_index].candidate(child_index)
    }

    fn take_candidate(&mut self, index: CandidateId) -> M {
        let (cursor_index, child_index) = self.discovered[index.index()];
        self.cursors[cursor_index].take_candidate(child_index)
    }

    fn apply_owned_candidate<D: Director<S>>(
        &mut self,
        index: CandidateId,
        score_director: &mut D,
    ) {
        let (cursor_index, child_index) = self.discovered[index.index()];
        self.cursors[cursor_index].apply_owned_candidate(child_index, score_director);
    }

    fn release_candidate(&mut self, index: CandidateId) -> bool {
        let Some(&(cursor_index, child_index)) = self.discovered.get(index.index()) else {
            return false;
        };
        self.cursors[cursor_index].release_candidate(child_index)
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        self.discovered
            .get(index.index())
            .map(|(cursor_index, _)| *cursor_index)
    }
}

/// Resource-aware facade over the canonical [`UnionScheduler`].
///
/// This deliberately shares no ordering implementation with a second union
/// cursor. It differs from [`VecUnionMoveCursor`] only in how the selected
/// child is asked for its next candidate: a caller-owned solve resource is
/// lent at that exact reachable pull boundary.
pub(crate) struct ResourceVecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
{
    cursors: Vec<C>,
    scheduler: UnionScheduler,
    discovered: Vec<(usize, CandidateId)>,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, C> ResourceVecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub(crate) fn new(
        cursors: Vec<C>,
        selection_order: UnionSelectionOrder,
        context: MoveStreamContext,
        weights: Vec<u64>,
    ) -> Self {
        Self {
            scheduler: UnionScheduler::new(cursors.len(), selection_order, context, weights),
            cursors,
            discovered: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

impl<S, M, C, Resources> ResourceMoveCursor<S, M, Resources> for ResourceVecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: ResourceMoveCursor<S, M, Resources>,
{
    fn next_candidate_with_resources<D: Director<S>>(
        &mut self,
        director: &mut D,
        resources: &mut Resources,
    ) -> Option<CandidateId> {
        let cursor_count = self.cursors.len();
        let (cursor_index, child_index) = self.scheduler.next_child(cursor_count, |index| {
            self.cursors[index].next_candidate_with_resources(director, resources)
        })?;
        let global_id = CandidateId::new(self.discovered.len());
        self.discovered.push((cursor_index, child_index));
        self.cursors[cursor_index]
            .candidate(child_index)
            .expect("resource union candidate must remain valid");
        Some(global_id)
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        let (cursor_index, child_index) = *self.discovered.get(index.index())?;
        self.cursors[cursor_index].candidate(child_index)
    }

    fn take_candidate(&mut self, index: CandidateId) -> M {
        let (cursor_index, child_index) = self.discovered[index.index()];
        self.cursors[cursor_index].take_candidate(child_index)
    }

    fn apply_owned_candidate<D: Director<S>>(
        &mut self,
        index: CandidateId,
        score_director: &mut D,
    ) {
        let (cursor_index, child_index) = self.discovered[index.index()];
        self.cursors[cursor_index].apply_owned_candidate(child_index, score_director);
    }

    fn release_candidate(&mut self, index: CandidateId) -> bool {
        let Some(&(cursor_index, child_index)) = self.discovered.get(index.index()) else {
            return false;
        };
        self.cursors[cursor_index].release_candidate(child_index)
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        self.discovered
            .get(index.index())
            .map(|(cursor_index, _)| *cursor_index)
    }
}

#[cfg(test)]
#[path = "vec_union/tests.rs"]
mod tests;
