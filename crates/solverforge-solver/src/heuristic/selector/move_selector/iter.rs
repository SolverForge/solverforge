pub struct MoveSelectorIter<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    cursor: C,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, C> MoveSelectorIter<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn new(cursor: C) -> Self {
        Self {
            cursor,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, C> Iterator for MoveSelectorIter<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    type Item = M;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.cursor.next_candidate()?;
        Some(self.cursor.take_candidate(id))
    }
}

/// A typed move selector that yields stable candidate indices plus borrowable
/// move views. Ownership is transferred only via `take_candidate`.
pub trait MoveSelector<S: PlanningSolution, M: Move<S>>: Send + Debug {
    type Cursor<'a>: MoveCursor<S, M> + 'a
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a>;

    fn iter_moves<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> MoveSelectorIter<S, M, Self::Cursor<'a>> {
        MoveSelectorIter::new(self.open_cursor(score_director))
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize;

    fn append_moves<D: Director<S>>(&self, score_director: &D, arena: &mut MoveArena<M>) {
        let mut cursor = self.open_cursor(score_director);
        for id in collect_cursor_indices::<S, M, _>(&mut cursor) {
            arena.push(cursor.take_candidate(id));
        }
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}
