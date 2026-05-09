use std::fmt::{self, Debug};

use solverforge_core::domain::PlanningSolution;

use crate::builder::list_selector::ListLeafSelector;
use crate::builder::scalar_selector::ScalarLeafSelector;
use crate::heuristic::r#move::{ListMoveUnion, MoveArena, ScalarMoveUnion};
use crate::heuristic::selector::decorator::MappedMoveCursor;
use crate::heuristic::selector::move_selector::{
    collect_cursor_indices, CandidateId, MoveCandidateRef, MoveCursor, MoveSelector,
    MoveStreamContext,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

use super::super::{ConflictRepairSelector, GroupedScalarSelector};
use super::move_union::NeighborhoodMove;

pub enum NeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    Scalar(ScalarLeafSelector<S>),
    List(ListLeafSelector<S, V, DM, IDM>),
    GroupedScalar(GroupedScalarSelector<S>),
    ConflictRepair(ConflictRepairSelector<S>),
}

fn wrap_scalar_neighborhood_move<S, V>(mov: ScalarMoveUnion<S, usize>) -> NeighborhoodMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    NeighborhoodMove::Scalar(mov)
}

fn wrap_list_neighborhood_move<S, V>(mov: ListMoveUnion<S, V>) -> NeighborhoodMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    NeighborhoodMove::List(mov)
}

pub enum NeighborhoodLeafCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    Scalar(
        MappedMoveCursor<
            S,
            ScalarMoveUnion<S, usize>,
            NeighborhoodMove<S, V>,
            <ScalarLeafSelector<S> as MoveSelector<S, ScalarMoveUnion<S, usize>>>::Cursor<'a>,
            fn(ScalarMoveUnion<S, usize>) -> NeighborhoodMove<S, V>,
        >,
    ),
    List(
        MappedMoveCursor<
            S,
            ListMoveUnion<S, V>,
            NeighborhoodMove<S, V>,
            <ListLeafSelector<S, V, DM, IDM> as MoveSelector<S, ListMoveUnion<S, V>>>::Cursor<'a>,
            fn(ListMoveUnion<S, V>) -> NeighborhoodMove<S, V>,
        >,
    ),
    ConflictRepair(
        MappedMoveCursor<
            S,
            ScalarMoveUnion<S, usize>,
            NeighborhoodMove<S, V>,
            <ConflictRepairSelector<S> as MoveSelector<S, ScalarMoveUnion<S, usize>>>::Cursor<'a>,
            fn(ScalarMoveUnion<S, usize>) -> NeighborhoodMove<S, V>,
        >,
    ),
    GroupedScalar(
        MappedMoveCursor<
            S,
            ScalarMoveUnion<S, usize>,
            NeighborhoodMove<S, V>,
            <GroupedScalarSelector<S> as MoveSelector<S, ScalarMoveUnion<S, usize>>>::Cursor<'a>,
            fn(ScalarMoveUnion<S, usize>) -> NeighborhoodMove<S, V>,
        >,
    ),
}

impl<S, V, DM, IDM> MoveCursor<S, NeighborhoodMove<S, V>>
    for NeighborhoodLeafCursor<'_, S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        match self {
            Self::Scalar(cursor) => cursor.next_candidate(),
            Self::List(cursor) => cursor.next_candidate(),
            Self::ConflictRepair(cursor) => cursor.next_candidate(),
            Self::GroupedScalar(cursor) => cursor.next_candidate(),
        }
    }

    fn candidate(
        &self,
        index: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, NeighborhoodMove<S, V>>> {
        match self {
            Self::Scalar(cursor) => cursor.candidate(index),
            Self::List(cursor) => cursor.candidate(index),
            Self::ConflictRepair(cursor) => cursor.candidate(index),
            Self::GroupedScalar(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: CandidateId) -> NeighborhoodMove<S, V> {
        match self {
            Self::Scalar(cursor) => cursor.take_candidate(index),
            Self::List(cursor) => cursor.take_candidate(index),
            Self::ConflictRepair(cursor) => cursor.take_candidate(index),
            Self::GroupedScalar(cursor) => cursor.take_candidate(index),
        }
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        match self {
            Self::Scalar(cursor) => cursor.selector_index(index),
            Self::List(cursor) => cursor.selector_index(index),
            Self::ConflictRepair(cursor) => cursor.selector_index(index),
            Self::GroupedScalar(cursor) => cursor.selector_index(index),
        }
    }
}

impl<S, V, DM, IDM> Debug for NeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(selector) => write!(f, "NeighborhoodLeaf::Scalar({selector:?})"),
            Self::List(selector) => write!(f, "NeighborhoodLeaf::List({selector:?})"),
            Self::ConflictRepair(selector) => {
                write!(f, "NeighborhoodLeaf::ConflictRepair({selector:?})")
            }
            Self::GroupedScalar(selector) => {
                write!(f, "NeighborhoodLeaf::GroupedScalar({selector:?})")
            }
        }
    }
}

impl<S, V, DM, IDM> MoveSelector<S, NeighborhoodMove<S, V>> for NeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    type Cursor<'a>
        = NeighborhoodLeafCursor<'a, S, V, DM, IDM>
    where
        Self: 'a;

    fn open_cursor<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
    ) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        match self {
            Self::Scalar(selector) => NeighborhoodLeafCursor::Scalar(MappedMoveCursor::new(
                selector.open_cursor_with_context(score_director, context),
                wrap_scalar_neighborhood_move::<S, V>,
            )),
            Self::List(selector) => NeighborhoodLeafCursor::List(MappedMoveCursor::new(
                selector.open_cursor_with_context(score_director, context),
                wrap_list_neighborhood_move::<S, V>,
            )),
            Self::ConflictRepair(selector) => {
                NeighborhoodLeafCursor::ConflictRepair(MappedMoveCursor::new(
                    selector.open_cursor_with_context(score_director, context),
                    wrap_scalar_neighborhood_move::<S, V>,
                ))
            }
            Self::GroupedScalar(selector) => {
                NeighborhoodLeafCursor::GroupedScalar(MappedMoveCursor::new(
                    selector.open_cursor_with_context(score_director, context),
                    wrap_scalar_neighborhood_move::<S, V>,
                ))
            }
        }
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Scalar(selector) => selector.size(score_director),
            Self::List(selector) => selector.size(score_director),
            Self::ConflictRepair(selector) => selector.size(score_director),
            Self::GroupedScalar(selector) => selector.size(score_director),
        }
    }

    fn append_moves<D: solverforge_scoring::Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<NeighborhoodMove<S, V>>,
    ) {
        let mut cursor = self.open_cursor(score_director);
        for id in collect_cursor_indices::<S, NeighborhoodMove<S, V>, _>(&mut cursor) {
            arena.push(cursor.take_candidate(id));
        }
    }
}
