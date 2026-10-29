/// Adapts an ordinary cursor to the resource-aware composition boundary.
///
/// It is deliberately explicit instead of a blanket implementation of
/// [`ResourceMoveCursor`]: compiled provider cursors need their own resource
/// lending implementation, and a blanket would overlap that specialization.
#[doc(hidden)]
pub struct UnitResourceCursor<C> {
    inner: C,
}

impl<C> UnitResourceCursor<C> {
    pub(crate) fn new(inner: C) -> Self {
        Self { inner }
    }
}

impl<S, M, C> ResourceMoveCursor<S, M, ()> for UnitResourceCursor<C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn next_candidate_with_resources<D: Director<S>>(&mut self, director: &mut D, _resources: &mut ()) -> Option<CandidateId> {
        self.inner.next_candidate(director.working_solution())
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
