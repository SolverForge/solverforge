use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{ListMoveImpl, MoveArena};
use crate::heuristic::selector::move_selector::{
    ListMoveKOptSelector, ListMoveListChangeSelector, ListMoveListRuinSelector,
    ListMoveNearbyKOptSelector, MoveSelector,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::{
    FromSolutionEntitySelector, ListMoveListReverseSelector, ListMoveListSwapSelector,
    ListMoveNearbyListChangeSelector, ListMoveNearbyListSwapSelector,
    ListMoveSubListChangeSelector, ListMoveSubListSwapSelector,
};

use super::super::context::IntraDistanceAdapter;

/// A monomorphized leaf selector for list planning variables.
///
/// Each variant wraps one of the available list move selector wrapper types.
/// Allows `VecUnionSelector<S, ListMoveImpl<S, V>, ListLeafSelector<S, V, DM, IDM>>` to have
/// a single concrete type regardless of configuration.
pub enum ListLeafSelector<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S>,
    IDM: CrossEntityDistanceMeter<S>,
{
    NearbyListChange(ListMoveNearbyListChangeSelector<S, V, DM, FromSolutionEntitySelector>),
    NearbyListSwap(ListMoveNearbyListSwapSelector<S, V, DM, FromSolutionEntitySelector>),
    ListReverse(ListMoveListReverseSelector<S, V, FromSolutionEntitySelector>),
    SubListChange(ListMoveSubListChangeSelector<S, V, FromSolutionEntitySelector>),
    KOpt(ListMoveKOptSelector<S, V, FromSolutionEntitySelector>),
    NearbyKOpt(
        ListMoveNearbyKOptSelector<S, V, IntraDistanceAdapter<IDM>, FromSolutionEntitySelector>,
    ),
    ListRuin(ListMoveListRuinSelector<S, V>),
    ListChange(ListMoveListChangeSelector<S, V, FromSolutionEntitySelector>),
    ListSwap(ListMoveListSwapSelector<S, V, FromSolutionEntitySelector>),
    SubListSwap(ListMoveSubListSwapSelector<S, V, FromSolutionEntitySelector>),
}

impl<S, V, DM, IDM> Debug for ListLeafSelector<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S>,
    IDM: CrossEntityDistanceMeter<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NearbyListChange(s) => write!(f, "ListLeafSelector::NearbyListChange({s:?})"),
            Self::NearbyListSwap(s) => write!(f, "ListLeafSelector::NearbyListSwap({s:?})"),
            Self::ListReverse(s) => write!(f, "ListLeafSelector::ListReverse({s:?})"),
            Self::SubListChange(s) => write!(f, "ListLeafSelector::SubListChange({s:?})"),
            Self::KOpt(s) => write!(f, "ListLeafSelector::KOpt({s:?})"),
            Self::NearbyKOpt(s) => write!(f, "ListLeafSelector::NearbyKOpt({s:?})"),
            Self::ListRuin(s) => write!(f, "ListLeafSelector::ListRuin({s:?})"),
            Self::ListChange(s) => write!(f, "ListLeafSelector::ListChange({s:?})"),
            Self::ListSwap(s) => write!(f, "ListLeafSelector::ListSwap({s:?})"),
            Self::SubListSwap(s) => write!(f, "ListLeafSelector::SubListSwap({s:?})"),
        }
    }
}

impl<S, V, DM, IDM> MoveSelector<S, ListMoveImpl<S, V>> for ListLeafSelector<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S>,
    IDM: CrossEntityDistanceMeter<S> + 'static,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = ListMoveImpl<S, V>> + 'a {
        enum ListLeafIter<A, B, C, DIter, E, F, G, H, I, J> {
            NearbyListChange(A),
            NearbyListSwap(B),
            ListReverse(C),
            SubListChange(DIter),
            KOpt(E),
            NearbyKOpt(F),
            ListRuin(G),
            ListChange(H),
            ListSwap(I),
            SubListSwap(J),
        }

        impl<T, A, B, C, DIter, E, F, G, H, I, J> Iterator
            for ListLeafIter<A, B, C, DIter, E, F, G, H, I, J>
        where
            A: Iterator<Item = T>,
            B: Iterator<Item = T>,
            C: Iterator<Item = T>,
            DIter: Iterator<Item = T>,
            E: Iterator<Item = T>,
            F: Iterator<Item = T>,
            G: Iterator<Item = T>,
            H: Iterator<Item = T>,
            I: Iterator<Item = T>,
            J: Iterator<Item = T>,
        {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Self::NearbyListChange(iter) => iter.next(),
                    Self::NearbyListSwap(iter) => iter.next(),
                    Self::ListReverse(iter) => iter.next(),
                    Self::SubListChange(iter) => iter.next(),
                    Self::KOpt(iter) => iter.next(),
                    Self::NearbyKOpt(iter) => iter.next(),
                    Self::ListRuin(iter) => iter.next(),
                    Self::ListChange(iter) => iter.next(),
                    Self::ListSwap(iter) => iter.next(),
                    Self::SubListSwap(iter) => iter.next(),
                }
            }
        }

        match self {
            Self::NearbyListChange(s) => {
                ListLeafIter::NearbyListChange(s.open_cursor(score_director))
            }
            Self::NearbyListSwap(s) => ListLeafIter::NearbyListSwap(s.open_cursor(score_director)),
            Self::ListReverse(s) => ListLeafIter::ListReverse(s.open_cursor(score_director)),
            Self::SubListChange(s) => ListLeafIter::SubListChange(s.open_cursor(score_director)),
            Self::KOpt(s) => ListLeafIter::KOpt(s.open_cursor(score_director)),
            Self::NearbyKOpt(s) => ListLeafIter::NearbyKOpt(s.open_cursor(score_director)),
            Self::ListRuin(s) => ListLeafIter::ListRuin(s.open_cursor(score_director)),
            Self::ListChange(s) => ListLeafIter::ListChange(s.open_cursor(score_director)),
            Self::ListSwap(s) => ListLeafIter::ListSwap(s.open_cursor(score_director)),
            Self::SubListSwap(s) => ListLeafIter::SubListSwap(s.open_cursor(score_director)),
        }
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::NearbyListChange(s) => s.size(score_director),
            Self::NearbyListSwap(s) => s.size(score_director),
            Self::ListReverse(s) => s.size(score_director),
            Self::SubListChange(s) => s.size(score_director),
            Self::KOpt(s) => s.size(score_director),
            Self::NearbyKOpt(s) => s.size(score_director),
            Self::ListRuin(s) => s.size(score_director),
            Self::ListChange(s) => s.size(score_director),
            Self::ListSwap(s) => s.size(score_director),
            Self::SubListSwap(s) => s.size(score_director),
        }
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ListMoveImpl<S, V>>,
    ) {
        match self {
            Self::NearbyListChange(s) => arena.extend(s.open_cursor(score_director)),
            Self::NearbyListSwap(s) => arena.extend(s.open_cursor(score_director)),
            Self::ListReverse(s) => arena.extend(s.open_cursor(score_director)),
            Self::SubListChange(s) => arena.extend(s.open_cursor(score_director)),
            Self::KOpt(s) => arena.extend(s.open_cursor(score_director)),
            Self::NearbyKOpt(s) => arena.extend(s.open_cursor(score_director)),
            Self::ListRuin(s) => arena.extend(s.open_cursor(score_director)),
            Self::ListChange(s) => arena.extend(s.open_cursor(score_director)),
            Self::ListSwap(s) => arena.extend(s.open_cursor(score_director)),
            Self::SubListSwap(s) => arena.extend(s.open_cursor(score_director)),
        }
    }
}
