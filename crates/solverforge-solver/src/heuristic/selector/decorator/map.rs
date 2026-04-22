use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{Move, MoveArena};
use crate::heuristic::selector::MoveSelector;

/// Maps one concrete move type into another without introducing type erasure.
pub struct MapMoveSelector<S, InM, OutM, Inner>
where
    S: PlanningSolution,
    InM: Move<S>,
    OutM: Move<S>,
    Inner: MoveSelector<S, InM>,
{
    inner: Inner,
    mapper: fn(InM) -> OutM,
    _phantom: PhantomData<(fn() -> S, fn() -> InM, fn() -> OutM)>,
}

impl<S, InM, OutM, Inner> MapMoveSelector<S, InM, OutM, Inner>
where
    S: PlanningSolution,
    InM: Move<S>,
    OutM: Move<S>,
    Inner: MoveSelector<S, InM>,
{
    pub fn new(inner: Inner, mapper: fn(InM) -> OutM) -> Self {
        Self {
            inner,
            mapper,
            _phantom: PhantomData,
        }
    }

    pub fn inner(&self) -> &Inner {
        &self.inner
    }
}

impl<S, InM, OutM, Inner> Debug for MapMoveSelector<S, InM, OutM, Inner>
where
    S: PlanningSolution,
    InM: Move<S>,
    OutM: Move<S>,
    Inner: MoveSelector<S, InM> + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapMoveSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, InM, OutM, Inner> MoveSelector<S, OutM> for MapMoveSelector<S, InM, OutM, Inner>
where
    S: PlanningSolution,
    InM: Move<S>,
    OutM: Move<S>,
    Inner: MoveSelector<S, InM>,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = OutM> + 'a {
        let mapper = self.mapper;
        self.inner.open_cursor(score_director).map(mapper)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }

    fn append_moves<D: Director<S>>(&self, score_director: &D, arena: &mut MoveArena<OutM>) {
        arena.extend(self.open_cursor(score_director));
    }

    fn is_never_ending(&self) -> bool {
        self.inner.is_never_ending()
    }
}
