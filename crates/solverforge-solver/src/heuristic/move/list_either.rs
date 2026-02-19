//! ListMoveImpl - a monomorphized union of all list-variable move types.
//!
//! This allows local search to combine all list move types in a single arena
//! without trait-object dispatch.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::{
    KOptMove, ListChangeMove, ListReverseMove, ListRuinMove, ListSwapMove, Move, SubListChangeMove,
    SubListSwapMove,
};

/// A monomorphized union of all list-variable move types.
///
/// Implements `Move<S>` by delegating to the inner variant.
/// Enables combining `ListChangeMoveSelector`, `ListSwapMoveSelector`,
/// `SubListChangeMoveSelector`, `SubListSwapMoveSelector`,
/// `ListReverseMoveSelector`, `KOptMoveSelector`, and `ListRuinMoveSelector`
/// without type erasure.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::ListMoveImpl;
/// ```
pub enum ListMoveImpl<S, V> {
    ListChange(ListChangeMove<S, V>),
    ListSwap(ListSwapMove<S, V>),
    SubListChange(SubListChangeMove<S, V>),
    SubListSwap(SubListSwapMove<S, V>),
    ListReverse(ListReverseMove<S, V>),
    KOpt(KOptMove<S, V>),
    ListRuin(ListRuinMove<S, V>),
}

impl<S, V: Clone> Clone for ListMoveImpl<S, V> {
    fn clone(&self) -> Self {
        match self {
            Self::ListChange(m) => Self::ListChange(*m),
            Self::ListSwap(m) => Self::ListSwap(*m),
            Self::SubListChange(m) => Self::SubListChange(*m),
            Self::SubListSwap(m) => Self::SubListSwap(*m),
            Self::ListReverse(m) => Self::ListReverse(*m),
            Self::KOpt(m) => Self::KOpt(m.clone()),
            Self::ListRuin(m) => Self::ListRuin(m.clone()),
        }
    }
}

impl<S, V: Debug> Debug for ListMoveImpl<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ListChange(m) => m.fmt(f),
            Self::ListSwap(m) => m.fmt(f),
            Self::SubListChange(m) => m.fmt(f),
            Self::SubListSwap(m) => m.fmt(f),
            Self::ListReverse(m) => m.fmt(f),
            Self::KOpt(m) => m.fmt(f),
            Self::ListRuin(m) => m.fmt(f),
        }
    }
}

impl<S, V> Move<S> for ListMoveImpl<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::ListChange(m) => m.is_doable(score_director),
            Self::ListSwap(m) => m.is_doable(score_director),
            Self::SubListChange(m) => m.is_doable(score_director),
            Self::SubListSwap(m) => m.is_doable(score_director),
            Self::ListReverse(m) => m.is_doable(score_director),
            Self::KOpt(m) => m.is_doable(score_director),
            Self::ListRuin(m) => m.is_doable(score_director),
        }
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        match self {
            Self::ListChange(m) => m.do_move(score_director),
            Self::ListSwap(m) => m.do_move(score_director),
            Self::SubListChange(m) => m.do_move(score_director),
            Self::SubListSwap(m) => m.do_move(score_director),
            Self::ListReverse(m) => m.do_move(score_director),
            Self::KOpt(m) => m.do_move(score_director),
            Self::ListRuin(m) => m.do_move(score_director),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::ListChange(m) => m.descriptor_index(),
            Self::ListSwap(m) => m.descriptor_index(),
            Self::SubListChange(m) => m.descriptor_index(),
            Self::SubListSwap(m) => m.descriptor_index(),
            Self::ListReverse(m) => m.descriptor_index(),
            Self::KOpt(m) => m.descriptor_index(),
            Self::ListRuin(m) => m.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::ListChange(m) => m.entity_indices(),
            Self::ListSwap(m) => m.entity_indices(),
            Self::SubListChange(m) => m.entity_indices(),
            Self::SubListSwap(m) => m.entity_indices(),
            Self::ListReverse(m) => m.entity_indices(),
            Self::KOpt(m) => m.entity_indices(),
            Self::ListRuin(m) => m.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::ListChange(m) => m.variable_name(),
            Self::ListSwap(m) => m.variable_name(),
            Self::SubListChange(m) => m.variable_name(),
            Self::SubListSwap(m) => m.variable_name(),
            Self::ListReverse(m) => m.variable_name(),
            Self::KOpt(m) => m.variable_name(),
            Self::ListRuin(m) => m.variable_name(),
        }
    }
}
