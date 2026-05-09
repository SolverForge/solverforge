use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{CandidateId, MoveCandidateRef, MoveCursor};

#[derive(Debug)]
pub struct LimitedMoveCursor<C> {
    inner: C,
    limit: usize,
    yielded: usize,
}

impl<C> LimitedMoveCursor<C> {
    pub(crate) fn new(inner: C, limit: usize) -> Self {
        Self {
            inner,
            limit,
            yielded: 0,
        }
    }
}

impl<S, M, C> MoveCursor<S, M> for LimitedMoveCursor<C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        if self.yielded >= self.limit {
            return None;
        }
        let id = self.inner.next_candidate()?;
        self.yielded += 1;
        Some(id)
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.inner.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> M {
        self.inner.take_candidate(id)
    }

    fn selector_index(&self, id: CandidateId) -> Option<usize> {
        self.inner.selector_index(id)
    }
}

#[cfg(test)]
#[path = "limited/tests.rs"]
mod tests;
