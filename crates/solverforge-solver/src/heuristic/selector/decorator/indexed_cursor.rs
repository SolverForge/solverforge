use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveCandidateRef, MoveCursor};

pub struct IndexedMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    inner: C,
    indices: Vec<usize>,
    next_outer_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, C> IndexedMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    pub fn new(inner: C, indices: Vec<usize>) -> Self {
        Self {
            inner,
            indices,
            next_outer_index: 0,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, C> MoveCursor<S, M> for IndexedMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn next_candidate(&mut self) -> Option<(usize, MoveCandidateRef<'_, S, M>)> {
        let outer_index = self.next_outer_index;
        let child_index = *self.indices.get(outer_index)?;
        self.next_outer_index += 1;
        let candidate = self
            .inner
            .candidate(child_index)
            .expect("indexed cursor candidate must remain valid");
        Some((outer_index, candidate))
    }

    fn candidate(&self, index: usize) -> Option<MoveCandidateRef<'_, S, M>> {
        let child_index = *self.indices.get(index)?;
        self.inner.candidate(child_index)
    }

    fn take_candidate(&mut self, index: usize) -> M {
        let child_index = self.indices[index];
        self.inner.take_candidate(child_index)
    }
}
