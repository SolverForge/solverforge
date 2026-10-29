pub(crate) struct CandidateStore<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    positions: Vec<Option<usize>>,
    live: Vec<StoredCandidate<M>>,
    _phantom: PhantomData<fn() -> S>,
}

struct StoredCandidate<M> {
    id: CandidateId,
    mov: M,
}

impl<S, M> CandidateStore<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub(crate) fn new() -> Self {
        Self {
            positions: Vec::new(),
            live: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Drops every stored candidate while retaining both backing
    /// capacities, so a reused store does not re-grow its vectors.
    pub(crate) fn reset(&mut self) {
        self.positions.clear();
        self.live.clear();
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            positions: Vec::with_capacity(capacity),
            live: Vec::with_capacity(capacity),
            _phantom: PhantomData,
        }
    }

    pub(crate) fn push(&mut self, mov: M) -> CandidateId {
        let id = CandidateId::new(self.positions.len());
        self.positions.push(Some(self.live.len()));
        self.live.push(StoredCandidate { id, mov });
        id
    }

    #[cfg(test)]
    pub(crate) fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = M>,
    {
        for mov in iter {
            self.push(mov);
        }
    }

    pub(crate) fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        let position = self.positions.get(id.index()).copied().flatten()?;
        self.live
            .get(position)
            .map(|stored| MoveCandidateRef::Borrowed(&stored.mov))
    }

    pub(crate) fn take_candidate(&mut self, id: CandidateId) -> M {
        let position = self
            .positions
            .get_mut(id.index())
            .and_then(Option::take)
            .expect("move cursor candidate id must remain valid");
        let stored = self.live.swap_remove(position);
        debug_assert_eq!(stored.id, id);
        if let Some(moved) = self.live.get(position) {
            self.positions[moved.id.index()] = Some(position);
        }
        stored.mov
    }

    pub(crate) fn release_candidate(&mut self, id: CandidateId) -> bool {
        if self
            .positions
            .get(id.index())
            .is_none_or(Option::is_none)
        {
            return false;
        }
        drop(self.take_candidate(id));
        true
    }

    pub(crate) fn len(&self) -> usize {
        self.positions.len()
    }

}

impl<S, M> Default for CandidateStore<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct ArenaMoveCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    store: CandidateStore<S, M>,
    next_index: usize,
}

impl<S, M> ArenaMoveCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub(crate) fn new() -> Self {
        Self {
            store: CandidateStore::new(),
            next_index: 0,
        }
    }

    #[cfg(test)]
    pub(crate) fn from_moves<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = M>,
    {
        let mut cursor = Self::new();
        cursor.extend(iter);
        cursor
    }

    #[cfg(test)]
    pub(crate) fn push(&mut self, mov: M) -> CandidateId {
        self.store.push(mov)
    }

    #[cfg(test)]
    pub(crate) fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = M>,
    {
        self.store.extend(iter);
    }

}

impl<S, M> Default for ArenaMoveCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S, M> Debug for ArenaMoveCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S> + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArenaMoveCursor")
            .field("move_count", &self.store.len())
            .field("next_index", &self.next_index)
            .finish()
    }
}

impl<S, M> MoveCursor<S, M> for ArenaMoveCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn next_candidate(&mut self, _solution: &S) -> Option<CandidateId> {
        while self.next_index < self.store.len() {
            let id = CandidateId::new(self.next_index);
            self.next_index += 1;
            if self.store.candidate(id).is_some() {
                return Some(id);
            }
        }
        None
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> M {
        self.store.take_candidate(id)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
    }
}
