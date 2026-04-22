use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{
    KOptMove, ListChangeMove, ListMoveUnion, ListReverseMove, ListRuinMove, ListSwapMove,
    MoveArena, SublistChangeMove, SublistSwapMove,
};
use crate::heuristic::selector::decorator::MapMoveSelector;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::{
    FromSolutionEntitySelector, KOptMoveSelector, ListChangeMoveSelector, ListReverseMoveSelector,
    ListRuinMoveSelector, ListSwapMoveSelector, MoveSelector, NearbyKOptMoveSelector,
    NearbyListChangeMoveSelector, NearbyListSwapMoveSelector, SublistChangeMoveSelector,
    SublistSwapMoveSelector,
};

use super::super::context::IntraDistanceAdapter;

type NearbyListChangeLeafSelector<S, V, DM> = MapMoveSelector<
    S,
    ListChangeMove<S, V>,
    ListMoveUnion<S, V>,
    NearbyListChangeMoveSelector<S, V, DM, FromSolutionEntitySelector>,
>;
type NearbyListSwapLeafSelector<S, V, DM> = MapMoveSelector<
    S,
    ListSwapMove<S, V>,
    ListMoveUnion<S, V>,
    NearbyListSwapMoveSelector<S, V, DM, FromSolutionEntitySelector>,
>;
type ListReverseLeafSelector<S, V> = MapMoveSelector<
    S,
    ListReverseMove<S, V>,
    ListMoveUnion<S, V>,
    ListReverseMoveSelector<S, V, FromSolutionEntitySelector>,
>;
type SublistChangeLeafSelector<S, V> = MapMoveSelector<
    S,
    SublistChangeMove<S, V>,
    ListMoveUnion<S, V>,
    SublistChangeMoveSelector<S, V, FromSolutionEntitySelector>,
>;
type KOptLeafSelector<S, V> = MapMoveSelector<
    S,
    KOptMove<S, V>,
    ListMoveUnion<S, V>,
    KOptMoveSelector<S, V, FromSolutionEntitySelector>,
>;
type NearbyKOptLeafSelector<S, V, IDM> = MapMoveSelector<
    S,
    KOptMove<S, V>,
    ListMoveUnion<S, V>,
    NearbyKOptMoveSelector<S, V, IntraDistanceAdapter<IDM>, FromSolutionEntitySelector>,
>;
type ListRuinLeafSelector<S, V> =
    MapMoveSelector<S, ListRuinMove<S, V>, ListMoveUnion<S, V>, ListRuinMoveSelector<S, V>>;
type ListChangeLeafSelector<S, V> = MapMoveSelector<
    S,
    ListChangeMove<S, V>,
    ListMoveUnion<S, V>,
    ListChangeMoveSelector<S, V, FromSolutionEntitySelector>,
>;
type ListSwapLeafSelector<S, V> = MapMoveSelector<
    S,
    ListSwapMove<S, V>,
    ListMoveUnion<S, V>,
    ListSwapMoveSelector<S, V, FromSolutionEntitySelector>,
>;
type SublistSwapLeafSelector<S, V> = MapMoveSelector<
    S,
    SublistSwapMove<S, V>,
    ListMoveUnion<S, V>,
    SublistSwapMoveSelector<S, V, FromSolutionEntitySelector>,
>;

/// A monomorphized leaf selector for list planning variables.
///
/// Each variant wraps one of the available list move selector wrapper types.
/// Allows `VecUnionSelector<S, ListMoveUnion<S, V>, ListLeafSelector<S, V, DM, IDM>>` to have
/// a single concrete type regardless of configuration.
pub enum ListLeafSelector<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S>,
    IDM: CrossEntityDistanceMeter<S> + 'static,
{
    NearbyListChange(NearbyListChangeLeafSelector<S, V, DM>),
    NearbyListSwap(NearbyListSwapLeafSelector<S, V, DM>),
    ListReverse(ListReverseLeafSelector<S, V>),
    SublistChange(SublistChangeLeafSelector<S, V>),
    KOpt(KOptLeafSelector<S, V>),
    NearbyKOpt(NearbyKOptLeafSelector<S, V, IDM>),
    ListRuin(ListRuinLeafSelector<S, V>),
    ListChange(ListChangeLeafSelector<S, V>),
    ListSwap(ListSwapLeafSelector<S, V>),
    SublistSwap(SublistSwapLeafSelector<S, V>),
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
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = ListMoveUnion<S, V>> + 'a {
        enum ListLeafIter<A, B, C, DIter, E, F, G, H, I, J> {
            NearbyListChange(A),
            NearbyListSwap(B),
            ListReverse(C),
            SublistChange(DIter),
            KOpt(E),
            NearbyKOpt(F),
            ListRuin(G),
            ListChange(H),
            ListSwap(I),
            SublistSwap(J),
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
                    Self::SublistChange(iter) => iter.next(),
                    Self::KOpt(iter) => iter.next(),
                    Self::NearbyKOpt(iter) => iter.next(),
                    Self::ListRuin(iter) => iter.next(),
                    Self::ListChange(iter) => iter.next(),
                    Self::ListSwap(iter) => iter.next(),
                    Self::SublistSwap(iter) => iter.next(),
                }
            }
        }

        match self {
            Self::NearbyListChange(s) => {
                ListLeafIter::NearbyListChange(s.open_cursor(score_director))
            }
            Self::NearbyListSwap(s) => ListLeafIter::NearbyListSwap(s.open_cursor(score_director)),
            Self::ListReverse(s) => ListLeafIter::ListReverse(s.open_cursor(score_director)),
            Self::SublistChange(s) => ListLeafIter::SublistChange(s.open_cursor(score_director)),
            Self::KOpt(s) => ListLeafIter::KOpt(s.open_cursor(score_director)),
            Self::NearbyKOpt(s) => ListLeafIter::NearbyKOpt(s.open_cursor(score_director)),
            Self::ListRuin(s) => ListLeafIter::ListRuin(s.open_cursor(score_director)),
            Self::ListChange(s) => ListLeafIter::ListChange(s.open_cursor(score_director)),
            Self::ListSwap(s) => ListLeafIter::ListSwap(s.open_cursor(score_director)),
            Self::SublistSwap(s) => ListLeafIter::SublistSwap(s.open_cursor(score_director)),
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

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ListMoveUnion<S, V>>,
    ) {
        match self {
            Self::NearbyListChange(s) => arena.extend(s.open_cursor(score_director)),
            Self::NearbyListSwap(s) => arena.extend(s.open_cursor(score_director)),
            Self::ListReverse(s) => arena.extend(s.open_cursor(score_director)),
            Self::SublistChange(s) => arena.extend(s.open_cursor(score_director)),
            Self::KOpt(s) => arena.extend(s.open_cursor(score_director)),
            Self::NearbyKOpt(s) => arena.extend(s.open_cursor(score_director)),
            Self::ListRuin(s) => arena.extend(s.open_cursor(score_director)),
            Self::ListChange(s) => arena.extend(s.open_cursor(score_director)),
            Self::ListSwap(s) => arena.extend(s.open_cursor(score_director)),
            Self::SublistSwap(s) => arena.extend(s.open_cursor(score_director)),
        }
    }
}
