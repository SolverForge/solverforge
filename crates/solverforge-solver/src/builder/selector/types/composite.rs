use std::fmt::{self, Debug};

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::selector::decorator::{
    CartesianProductCursor, CartesianProductSelector, LimitedMoveCursor,
};
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

use super::{LeafSelector, NeighborhoodMove};

pub enum Neighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    Flat(LeafSelector<S, V, DM, IDM>),
    Limited {
        selector: LeafSelector<S, V, DM, IDM>,
        selected_count_limit: usize,
    },
    Cartesian(CartesianNeighborhoodSelector<S, V, DM, IDM>),
}

pub enum CartesianChildSelector<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    Flat(LeafSelector<S, V, DM, IDM>),
    Limited {
        selector: LeafSelector<S, V, DM, IDM>,
        selected_count_limit: usize,
    },
}

type CartesianNeighborhoodSelector<S, V, DM, IDM> = CartesianProductSelector<
    S,
    NeighborhoodMove<S, V>,
    CartesianChildSelector<S, V, DM, IDM>,
    CartesianChildSelector<S, V, DM, IDM>,
>;

pub enum CartesianChildCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    Flat(<LeafSelector<S, V, DM, IDM> as MoveSelector<S, NeighborhoodMove<S, V>>>::Cursor<'a>),
    Limited(
        LimitedMoveCursor<
            <LeafSelector<S, V, DM, IDM> as MoveSelector<S, NeighborhoodMove<S, V>>>::Cursor<'a>,
        >,
    ),
}

impl<S, V, DM, IDM> MoveCursor<S, NeighborhoodMove<S, V>>
    for CartesianChildCursor<'_, S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        match self {
            Self::Flat(cursor) => cursor.next_candidate(),
            Self::Limited(cursor) => cursor.next_candidate(),
        }
    }

    fn candidate(
        &self,
        index: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, NeighborhoodMove<S, V>>> {
        match self {
            Self::Flat(cursor) => cursor.candidate(index),
            Self::Limited(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: CandidateId) -> NeighborhoodMove<S, V> {
        match self {
            Self::Flat(cursor) => cursor.take_candidate(index),
            Self::Limited(cursor) => cursor.take_candidate(index),
        }
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        match self {
            Self::Flat(cursor) => cursor.selector_index(index),
            Self::Limited(cursor) => cursor.selector_index(index),
        }
    }
}

pub enum NeighborhoodCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    Flat(<LeafSelector<S, V, DM, IDM> as MoveSelector<S, NeighborhoodMove<S, V>>>::Cursor<'a>),
    Limited(
        LimitedMoveCursor<
            <LeafSelector<S, V, DM, IDM> as MoveSelector<S, NeighborhoodMove<S, V>>>::Cursor<'a>,
        >,
    ),
    Cartesian(CartesianProductCursor<S, NeighborhoodMove<S, V>>),
}

impl<S, V, DM, IDM> MoveCursor<S, NeighborhoodMove<S, V>> for NeighborhoodCursor<'_, S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        match self {
            Self::Flat(cursor) => cursor.next_candidate(),
            Self::Limited(cursor) => cursor.next_candidate(),
            Self::Cartesian(cursor) => cursor.next_candidate(),
        }
    }

    fn candidate(
        &self,
        index: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, NeighborhoodMove<S, V>>> {
        match self {
            Self::Flat(cursor) => cursor.candidate(index),
            Self::Limited(cursor) => cursor.candidate(index),
            Self::Cartesian(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: CandidateId) -> NeighborhoodMove<S, V> {
        match self {
            Self::Flat(cursor) => cursor.take_candidate(index),
            Self::Limited(cursor) => cursor.take_candidate(index),
            Self::Cartesian(cursor) => cursor.take_candidate(index),
        }
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        match self {
            Self::Flat(cursor) => cursor.selector_index(index),
            Self::Limited(cursor) => cursor.selector_index(index),
            Self::Cartesian(cursor) => cursor.selector_index(index),
        }
    }
}

impl<S, V, DM, IDM> Debug for Neighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flat(selector) => write!(f, "Neighborhood::Flat({selector:?})"),
            Self::Limited {
                selector,
                selected_count_limit,
            } => f
                .debug_struct("Neighborhood::Limited")
                .field("selector", selector)
                .field("selected_count_limit", selected_count_limit)
                .finish(),
            Self::Cartesian(selector) => write!(f, "Neighborhood::Cartesian({selector:?})"),
        }
    }
}

impl<S, V, DM, IDM> MoveSelector<S, NeighborhoodMove<S, V>> for Neighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    type Cursor<'a>
        = NeighborhoodCursor<'a, S, V, DM, IDM>
    where
        Self: 'a;

    fn open_cursor<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
    ) -> Self::Cursor<'a> {
        match self {
            Self::Flat(selector) => NeighborhoodCursor::Flat(selector.open_cursor(score_director)),
            Self::Limited {
                selector,
                selected_count_limit,
            } => NeighborhoodCursor::Limited(LimitedMoveCursor::new(
                selector.open_cursor(score_director),
                *selected_count_limit,
            )),
            Self::Cartesian(selector) => {
                NeighborhoodCursor::Cartesian(selector.open_cursor(score_director))
            }
        }
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Flat(selector) => selector.size(score_director),
            Self::Limited {
                selector,
                selected_count_limit,
            } => selector.size(score_director).min(*selected_count_limit),
            Self::Cartesian(selector) => selector.size(score_director),
        }
    }
}

impl<S, V, DM, IDM> Debug for CartesianChildSelector<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flat(selector) => write!(f, "CartesianChildSelector::Flat({selector:?})"),
            Self::Limited {
                selector,
                selected_count_limit,
            } => f
                .debug_struct("CartesianChildSelector::Limited")
                .field("selector", selector)
                .field("selected_count_limit", selected_count_limit)
                .finish(),
        }
    }
}

impl<S, V, DM, IDM> MoveSelector<S, NeighborhoodMove<S, V>>
    for CartesianChildSelector<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    type Cursor<'a>
        = CartesianChildCursor<'a, S, V, DM, IDM>
    where
        Self: 'a;

    fn open_cursor<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
    ) -> Self::Cursor<'a> {
        match self {
            Self::Flat(selector) => {
                CartesianChildCursor::Flat(selector.open_cursor(score_director))
            }
            Self::Limited {
                selector,
                selected_count_limit,
            } => CartesianChildCursor::Limited(LimitedMoveCursor::new(
                selector.open_cursor(score_director),
                *selected_count_limit,
            )),
        }
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Flat(selector) => selector.size(score_director),
            Self::Limited {
                selector,
                selected_count_limit,
            } => selector.size(score_director).min(*selected_count_limit),
        }
    }
}
