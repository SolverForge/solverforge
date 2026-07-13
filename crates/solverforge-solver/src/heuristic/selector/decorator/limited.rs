use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, ResourceMoveCursor,
};

/// The one limit-accounting state used by ordinary and resource-aware cursors.
///
/// Resource-aware composition differs only in how a reachable child produces
/// its next candidate. The selected-count contract itself remains identical.
#[derive(Debug)]
struct LimitedCursorBudget {
    limit: usize,
    yielded: usize,
}

impl LimitedCursorBudget {
    fn new(limit: usize) -> Self {
        Self { limit, yielded: 0 }
    }

    fn can_pull(&self) -> bool {
        self.yielded < self.limit
    }

    fn record_pull(&mut self) {
        self.yielded += 1;
    }
}

/// The one selected-count cursor for ordinary and resource-aware children.
///
/// No candidates or resources are cached here; the execution owner lends the
/// resource at each reachable pull boundary. Ordinary users exercise the same
/// budget through its `MoveCursor` implementation.
#[derive(Debug)]
pub(crate) struct LimitedMoveCursor<C> {
    inner: C,
    budget: LimitedCursorBudget,
}

impl<C> LimitedMoveCursor<C> {
    pub(crate) fn new(inner: C, limit: usize) -> Self {
        Self {
            inner,
            budget: LimitedCursorBudget::new(limit),
        }
    }

    /// Returns the one wrapped cursor when a retained composition cursor is
    /// closed.  The recursive composition owns the inner stream state and
    /// must recover it exactly rather than reconstructing a child selector.
    pub(crate) fn into_inner(self) -> C {
        self.inner
    }
}

impl<S, M, C> MoveCursor<S, M> for LimitedMoveCursor<C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        if !self.budget.can_pull() {
            return None;
        }
        let id = self.inner.next_candidate()?;
        self.budget.record_pull();
        Some(id)
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.inner.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> M {
        self.inner.take_candidate(id)
    }

    fn apply_owned_candidate<D: Director<S>>(&mut self, id: CandidateId, score_director: &mut D) {
        self.inner.apply_owned_candidate(id, score_director);
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }

    fn selector_index(&self, id: CandidateId) -> Option<usize> {
        self.inner.selector_index(id)
    }
}

impl<S, M, C, Resources> ResourceMoveCursor<S, M, Resources> for LimitedMoveCursor<C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: ResourceMoveCursor<S, M, Resources>,
{
    fn next_candidate_with_resources(&mut self, resources: &mut Resources) -> Option<CandidateId> {
        if !self.budget.can_pull() {
            return None;
        }
        let id = self.inner.next_candidate_with_resources(resources)?;
        self.budget.record_pull();
        Some(id)
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.inner.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> M {
        self.inner.take_candidate(id)
    }

    fn apply_owned_candidate<D: Director<S>>(&mut self, id: CandidateId, score_director: &mut D) {
        self.inner.apply_owned_candidate(id, score_director);
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }

    fn selector_index(&self, id: CandidateId) -> Option<usize> {
        self.inner.selector_index(id)
    }
}

#[cfg(test)]
#[path = "limited/tests.rs"]
mod tests;
