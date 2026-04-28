use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor,
};

pub struct MappedMoveCursor<S, ChildMove, ParentMove, ChildCursor, Map>
where
    S: PlanningSolution,
    ChildMove: Move<S>,
    ParentMove: Move<S>,
    ChildCursor: MoveCursor<S, ChildMove>,
    Map: Fn(ChildMove) -> ParentMove,
{
    child: ChildCursor,
    map: Map,
    store: CandidateStore<S, ParentMove>,
    _phantom: PhantomData<fn() -> ChildMove>,
}

impl<S, ChildMove, ParentMove, ChildCursor, Map>
    MappedMoveCursor<S, ChildMove, ParentMove, ChildCursor, Map>
where
    S: PlanningSolution,
    ChildMove: Move<S>,
    ParentMove: Move<S>,
    ChildCursor: MoveCursor<S, ChildMove>,
    Map: Fn(ChildMove) -> ParentMove,
{
    pub fn new(child: ChildCursor, map: Map) -> Self {
        Self {
            child,
            map,
            store: CandidateStore::new(),
            _phantom: PhantomData,
        }
    }
}

impl<S, ChildMove, ParentMove, ChildCursor, Map> MoveCursor<S, ParentMove>
    for MappedMoveCursor<S, ChildMove, ParentMove, ChildCursor, Map>
where
    S: PlanningSolution,
    ChildMove: Move<S>,
    ParentMove: Move<S>,
    ChildCursor: MoveCursor<S, ChildMove>,
    Map: Fn(ChildMove) -> ParentMove,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        let child_id = self.child.next_candidate()?;
        let child_move = self.child.take_candidate(child_id);
        Some(self.store.push((self.map)(child_move)))
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ParentMove>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ParentMove {
        self.store.take_candidate(id)
    }
}
