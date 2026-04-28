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
    selector_indices: Vec<Option<usize>>,
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
            selector_indices: Vec::new(),
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
        let selector_index = self.child.selector_index(child_id);
        let child_move = self.child.take_candidate(child_id);
        let id = self.store.push((self.map)(child_move));
        self.selector_indices.push(selector_index);
        Some(id)
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ParentMove>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ParentMove {
        self.store.take_candidate(id)
    }

    fn selector_index(&self, id: CandidateId) -> Option<usize> {
        self.selector_indices.get(id.index()).copied().flatten()
    }
}
