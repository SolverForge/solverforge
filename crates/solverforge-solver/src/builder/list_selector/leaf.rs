use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::ListMoveUnion;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::{
    move_selector::ArenaMoveCursor, FromSolutionEntitySelector, KOptMoveSelector,
    ListChangeMoveSelector, ListReverseMoveSelector, ListRuinMoveSelector, ListSwapMoveSelector,
    MoveSelector, NearbyKOptMoveSelector, NearbyListChangeMoveSelector, NearbyListSwapMoveSelector,
    SublistChangeMoveSelector, SublistSwapMoveSelector,
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
        = ArenaMoveCursor<S, ListMoveUnion<S, V>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        match self {
            Self::NearbyListChange(s) => ArenaMoveCursor::from_moves(
                s.iter_moves(score_director).map(ListMoveUnion::ListChange),
            ),
            Self::NearbyListSwap(s) => ArenaMoveCursor::from_moves(
                s.iter_moves(score_director).map(ListMoveUnion::ListSwap),
            ),
            Self::ListReverse(s) => ArenaMoveCursor::from_moves(
                s.iter_moves(score_director).map(ListMoveUnion::ListReverse),
            ),
            Self::SublistChange(s) => ArenaMoveCursor::from_moves(
                s.iter_moves(score_director)
                    .map(ListMoveUnion::SublistChange),
            ),
            Self::KOpt(s) => {
                ArenaMoveCursor::from_moves(s.iter_moves(score_director).map(ListMoveUnion::KOpt))
            }
            Self::NearbyKOpt(s) => {
                ArenaMoveCursor::from_moves(s.iter_moves(score_director).map(ListMoveUnion::KOpt))
            }
            Self::ListRuin(s) => ArenaMoveCursor::from_moves(
                s.iter_moves(score_director).map(ListMoveUnion::ListRuin),
            ),
            Self::ListChange(s) => ArenaMoveCursor::from_moves(
                s.iter_moves(score_director).map(ListMoveUnion::ListChange),
            ),
            Self::ListSwap(s) => ArenaMoveCursor::from_moves(
                s.iter_moves(score_director).map(ListMoveUnion::ListSwap),
            ),
            Self::SublistSwap(s) => ArenaMoveCursor::from_moves(
                s.iter_moves(score_director).map(ListMoveUnion::SublistSwap),
            ),
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
