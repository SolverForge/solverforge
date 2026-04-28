use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{
    KOptMove, ListChangeMove, ListMoveUnion, ListReverseMove, ListRuinMove, ListSwapMove,
    SublistChangeMove, SublistSwapMove,
};
use crate::heuristic::selector::decorator::MappedMoveCursor;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::{
    move_selector::{CandidateId, MoveCandidateRef, MoveCursor},
    FromSolutionEntitySelector, KOptMoveSelector, ListChangeMoveSelector, ListReverseMoveSelector,
    ListRuinMoveSelector, ListSwapMoveSelector, MoveSelector, NearbyKOptMoveSelector,
    NearbyListChangeMoveSelector, NearbyListSwapMoveSelector, SublistChangeMoveSelector,
    SublistSwapMoveSelector,
};

use super::super::context::IntraDistanceAdapter;

/// A monomorphized leaf selector for list planning variables.
///
/// Each variant stores a concrete list selector and lifts its moves directly
/// into `ListMoveUnion` when the leaf cursor opens.
/// Allows `VecUnionSelector<S, ListMoveUnion<S, V>, ListLeafSelector<S, V, DM, IDM>>` to have
/// a single concrete type regardless of configuration.
pub enum ListLeafSelector<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S>,
    IDM: CrossEntityDistanceMeter<S> + 'static,
{
    NearbyListChange(NearbyListChangeMoveSelector<S, V, DM, FromSolutionEntitySelector>),
    NearbyListSwap(NearbyListSwapMoveSelector<S, V, DM, FromSolutionEntitySelector>),
    ListReverse(ListReverseMoveSelector<S, V, FromSolutionEntitySelector>),
    SublistChange(SublistChangeMoveSelector<S, V, FromSolutionEntitySelector>),
    KOpt(KOptMoveSelector<S, V, FromSolutionEntitySelector>),
    NearbyKOpt(NearbyKOptMoveSelector<S, V, IntraDistanceAdapter<IDM>, FromSolutionEntitySelector>),
    ListRuin(ListRuinMoveSelector<S, V>),
    ListChange(ListChangeMoveSelector<S, V, FromSolutionEntitySelector>),
    ListSwap(ListSwapMoveSelector<S, V, FromSolutionEntitySelector>),
    SublistSwap(SublistSwapMoveSelector<S, V, FromSolutionEntitySelector>),
}

fn wrap_list_change_move<S, V>(mov: ListChangeMove<S, V>) -> ListMoveUnion<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    ListMoveUnion::ListChange(mov)
}

fn wrap_list_swap_move<S, V>(mov: ListSwapMove<S, V>) -> ListMoveUnion<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    ListMoveUnion::ListSwap(mov)
}

fn wrap_list_reverse_move<S, V>(mov: ListReverseMove<S, V>) -> ListMoveUnion<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    ListMoveUnion::ListReverse(mov)
}

fn wrap_sublist_change_move<S, V>(mov: SublistChangeMove<S, V>) -> ListMoveUnion<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    ListMoveUnion::SublistChange(mov)
}

fn wrap_k_opt_move<S, V>(mov: KOptMove<S, V>) -> ListMoveUnion<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    ListMoveUnion::KOpt(mov)
}

fn wrap_list_ruin_move<S, V>(mov: ListRuinMove<S, V>) -> ListMoveUnion<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    ListMoveUnion::ListRuin(mov)
}

fn wrap_sublist_swap_move<S, V>(mov: SublistSwapMove<S, V>) -> ListMoveUnion<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    ListMoveUnion::SublistSwap(mov)
}

#[allow(clippy::large_enum_variant)]
pub enum ListLeafCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + 'a,
    IDM: CrossEntityDistanceMeter<S> + 'static + 'a,
{
    NearbyListChange(MappedMoveCursor<
        S,
        ListChangeMove<S, V>,
        ListMoveUnion<S, V>,
        <NearbyListChangeMoveSelector<S, V, DM, FromSolutionEntitySelector> as MoveSelector<
            S,
            ListChangeMove<S, V>,
        >>::Cursor<'a>,
        fn(ListChangeMove<S, V>) -> ListMoveUnion<S, V>,
    >),
    NearbyListSwap(MappedMoveCursor<
        S,
        ListSwapMove<S, V>,
        ListMoveUnion<S, V>,
        <NearbyListSwapMoveSelector<S, V, DM, FromSolutionEntitySelector> as MoveSelector<
            S,
            ListSwapMove<S, V>,
        >>::Cursor<'a>,
        fn(ListSwapMove<S, V>) -> ListMoveUnion<S, V>,
    >),
    ListReverse(MappedMoveCursor<
        S,
        ListReverseMove<S, V>,
        ListMoveUnion<S, V>,
        <ListReverseMoveSelector<S, V, FromSolutionEntitySelector> as MoveSelector<
            S,
            ListReverseMove<S, V>,
        >>::Cursor<'a>,
        fn(ListReverseMove<S, V>) -> ListMoveUnion<S, V>,
    >),
    SublistChange(MappedMoveCursor<
        S,
        SublistChangeMove<S, V>,
        ListMoveUnion<S, V>,
        <SublistChangeMoveSelector<S, V, FromSolutionEntitySelector> as MoveSelector<
            S,
            SublistChangeMove<S, V>,
        >>::Cursor<'a>,
        fn(SublistChangeMove<S, V>) -> ListMoveUnion<S, V>,
    >),
    KOpt(MappedMoveCursor<
        S,
        KOptMove<S, V>,
        ListMoveUnion<S, V>,
        <KOptMoveSelector<S, V, FromSolutionEntitySelector> as MoveSelector<
            S,
            KOptMove<S, V>,
        >>::Cursor<'a>,
        fn(KOptMove<S, V>) -> ListMoveUnion<S, V>,
    >),
    NearbyKOpt(MappedMoveCursor<
        S,
        KOptMove<S, V>,
        ListMoveUnion<S, V>,
        <NearbyKOptMoveSelector<
            S,
            V,
            IntraDistanceAdapter<IDM>,
            FromSolutionEntitySelector,
        > as MoveSelector<S, KOptMove<S, V>>>::Cursor<'a>,
        fn(KOptMove<S, V>) -> ListMoveUnion<S, V>,
    >),
    ListRuin(MappedMoveCursor<
        S,
        ListRuinMove<S, V>,
        ListMoveUnion<S, V>,
        <ListRuinMoveSelector<S, V> as MoveSelector<S, ListRuinMove<S, V>>>::Cursor<'a>,
        fn(ListRuinMove<S, V>) -> ListMoveUnion<S, V>,
    >),
    ListChange(MappedMoveCursor<
        S,
        ListChangeMove<S, V>,
        ListMoveUnion<S, V>,
        <ListChangeMoveSelector<S, V, FromSolutionEntitySelector> as MoveSelector<
            S,
            ListChangeMove<S, V>,
        >>::Cursor<'a>,
        fn(ListChangeMove<S, V>) -> ListMoveUnion<S, V>,
    >),
    ListSwap(MappedMoveCursor<
        S,
        ListSwapMove<S, V>,
        ListMoveUnion<S, V>,
        <ListSwapMoveSelector<S, V, FromSolutionEntitySelector> as MoveSelector<
            S,
            ListSwapMove<S, V>,
        >>::Cursor<'a>,
        fn(ListSwapMove<S, V>) -> ListMoveUnion<S, V>,
    >),
    SublistSwap(MappedMoveCursor<
        S,
        SublistSwapMove<S, V>,
        ListMoveUnion<S, V>,
        <SublistSwapMoveSelector<S, V, FromSolutionEntitySelector> as MoveSelector<
            S,
            SublistSwapMove<S, V>,
        >>::Cursor<'a>,
        fn(SublistSwapMove<S, V>) -> ListMoveUnion<S, V>,
    >),
}

impl<'a, S, V, DM, IDM> MoveCursor<S, ListMoveUnion<S, V>> for ListLeafCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + 'a,
    IDM: CrossEntityDistanceMeter<S> + 'static + 'a,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        match self {
            Self::NearbyListChange(cursor) => cursor.next_candidate(),
            Self::NearbyListSwap(cursor) => cursor.next_candidate(),
            Self::ListReverse(cursor) => cursor.next_candidate(),
            Self::SublistChange(cursor) => cursor.next_candidate(),
            Self::KOpt(cursor) => cursor.next_candidate(),
            Self::NearbyKOpt(cursor) => cursor.next_candidate(),
            Self::ListRuin(cursor) => cursor.next_candidate(),
            Self::ListChange(cursor) => cursor.next_candidate(),
            Self::ListSwap(cursor) => cursor.next_candidate(),
            Self::SublistSwap(cursor) => cursor.next_candidate(),
        }
    }

    fn candidate(
        &self,
        index: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, ListMoveUnion<S, V>>> {
        match self {
            Self::NearbyListChange(cursor) => cursor.candidate(index),
            Self::NearbyListSwap(cursor) => cursor.candidate(index),
            Self::ListReverse(cursor) => cursor.candidate(index),
            Self::SublistChange(cursor) => cursor.candidate(index),
            Self::KOpt(cursor) => cursor.candidate(index),
            Self::NearbyKOpt(cursor) => cursor.candidate(index),
            Self::ListRuin(cursor) => cursor.candidate(index),
            Self::ListChange(cursor) => cursor.candidate(index),
            Self::ListSwap(cursor) => cursor.candidate(index),
            Self::SublistSwap(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: CandidateId) -> ListMoveUnion<S, V> {
        match self {
            Self::NearbyListChange(cursor) => cursor.take_candidate(index),
            Self::NearbyListSwap(cursor) => cursor.take_candidate(index),
            Self::ListReverse(cursor) => cursor.take_candidate(index),
            Self::SublistChange(cursor) => cursor.take_candidate(index),
            Self::KOpt(cursor) => cursor.take_candidate(index),
            Self::NearbyKOpt(cursor) => cursor.take_candidate(index),
            Self::ListRuin(cursor) => cursor.take_candidate(index),
            Self::ListChange(cursor) => cursor.take_candidate(index),
            Self::ListSwap(cursor) => cursor.take_candidate(index),
            Self::SublistSwap(cursor) => cursor.take_candidate(index),
        }
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        match self {
            Self::NearbyListChange(cursor) => cursor.selector_index(index),
            Self::NearbyListSwap(cursor) => cursor.selector_index(index),
            Self::ListReverse(cursor) => cursor.selector_index(index),
            Self::SublistChange(cursor) => cursor.selector_index(index),
            Self::KOpt(cursor) => cursor.selector_index(index),
            Self::NearbyKOpt(cursor) => cursor.selector_index(index),
            Self::ListRuin(cursor) => cursor.selector_index(index),
            Self::ListChange(cursor) => cursor.selector_index(index),
            Self::ListSwap(cursor) => cursor.selector_index(index),
            Self::SublistSwap(cursor) => cursor.selector_index(index),
        }
    }
}

impl<S, V, DM, IDM> Debug for ListLeafSelector<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S>,
    IDM: CrossEntityDistanceMeter<S> + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NearbyListChange(s) => write!(f, "ListLeafSelector::NearbyListChange({s:?})"),
            Self::NearbyListSwap(s) => write!(f, "ListLeafSelector::NearbyListSwap({s:?})"),
            Self::ListReverse(s) => write!(f, "ListLeafSelector::ListReverse({s:?})"),
            Self::SublistChange(s) => write!(f, "ListLeafSelector::SublistChange({s:?})"),
            Self::KOpt(s) => write!(f, "ListLeafSelector::KOpt({s:?})"),
            Self::NearbyKOpt(s) => write!(f, "ListLeafSelector::NearbyKOpt({s:?})"),
            Self::ListRuin(s) => write!(f, "ListLeafSelector::ListRuin({s:?})"),
            Self::ListChange(s) => write!(f, "ListLeafSelector::ListChange({s:?})"),
            Self::ListSwap(s) => write!(f, "ListLeafSelector::ListSwap({s:?})"),
            Self::SublistSwap(s) => write!(f, "ListLeafSelector::SublistSwap({s:?})"),
        }
    }
}

impl<S, V, DM, IDM> MoveSelector<S, ListMoveUnion<S, V>> for ListLeafSelector<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S>,
    IDM: CrossEntityDistanceMeter<S> + 'static,
{
    type Cursor<'a>
        = ListLeafCursor<'a, S, V, DM, IDM>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        match self {
            Self::NearbyListChange(s) => ListLeafCursor::NearbyListChange(MappedMoveCursor::new(
                s.open_cursor(score_director),
                wrap_list_change_move::<S, V>,
            )),
            Self::NearbyListSwap(s) => ListLeafCursor::NearbyListSwap(MappedMoveCursor::new(
                s.open_cursor(score_director),
                wrap_list_swap_move::<S, V>,
            )),
            Self::ListReverse(s) => ListLeafCursor::ListReverse(MappedMoveCursor::new(
                s.open_cursor(score_director),
                wrap_list_reverse_move::<S, V>,
            )),
            Self::SublistChange(s) => ListLeafCursor::SublistChange(MappedMoveCursor::new(
                s.open_cursor(score_director),
                wrap_sublist_change_move::<S, V>,
            )),
            Self::KOpt(s) => ListLeafCursor::KOpt(MappedMoveCursor::new(
                s.open_cursor(score_director),
                wrap_k_opt_move::<S, V>,
            )),
            Self::NearbyKOpt(s) => ListLeafCursor::NearbyKOpt(MappedMoveCursor::new(
                s.open_cursor(score_director),
                wrap_k_opt_move::<S, V>,
            )),
            Self::ListRuin(s) => ListLeafCursor::ListRuin(MappedMoveCursor::new(
                s.open_cursor(score_director),
                wrap_list_ruin_move::<S, V>,
            )),
            Self::ListChange(s) => ListLeafCursor::ListChange(MappedMoveCursor::new(
                s.open_cursor(score_director),
                wrap_list_change_move::<S, V>,
            )),
            Self::ListSwap(s) => ListLeafCursor::ListSwap(MappedMoveCursor::new(
                s.open_cursor(score_director),
                wrap_list_swap_move::<S, V>,
            )),
            Self::SublistSwap(s) => ListLeafCursor::SublistSwap(MappedMoveCursor::new(
                s.open_cursor(score_director),
                wrap_sublist_swap_move::<S, V>,
            )),
        }
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::NearbyListChange(s) => s.size(score_director),
            Self::NearbyListSwap(s) => s.size(score_director),
            Self::ListReverse(s) => s.size(score_director),
            Self::SublistChange(s) => s.size(score_director),
            Self::KOpt(s) => s.size(score_director),
            Self::NearbyKOpt(s) => s.size(score_director),
            Self::ListRuin(s) => s.size(score_director),
            Self::ListChange(s) => s.size(score_director),
            Self::ListSwap(s) => s.size(score_director),
            Self::SublistSwap(s) => s.size(score_director),
        }
    }
}
