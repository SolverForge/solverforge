//! EitherMove - a monomorphized union of ChangeMove and SwapMove.
//!
//! This allows local search to use both move types without trait-object dispatch.
//! The construction phase uses ChangeMove directly, while local search uses
//! EitherMove<S, V> = ChangeMove | SwapMove.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::{ChangeMove, Move, SwapMove};

/// A monomorphized union of `ChangeMove` and `SwapMove`.
///
/// Implements `Move<S>` by delegating to the inner variant.
/// `Copy` when `V: Copy`, avoiding heap allocation in the move selector hot path.
pub enum EitherMove<S, V> {
    Change(ChangeMove<S, V>),
    Swap(SwapMove<S, V>),
}

impl<S, V: Clone> Clone for EitherMove<S, V> {
    fn clone(&self) -> Self {
        match self {
            Self::Change(m) => Self::Change(m.clone()),
            Self::Swap(m) => Self::Swap(m.clone()),
        }
    }
}

impl<S, V: Copy> Copy for EitherMove<S, V> {}

impl<S, V: Debug> Debug for EitherMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Change(m) => m.fmt(f),
            Self::Swap(m) => m.fmt(f),
        }
    }
}

impl<S, V> Move<S> for EitherMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::Change(m) => m.is_doable(score_director),
            Self::Swap(m) => m.is_doable(score_director),
        }
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        match self {
            Self::Change(m) => m.do_move(score_director),
            Self::Swap(m) => m.do_move(score_director),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Change(m) => m.descriptor_index(),
            Self::Swap(m) => m.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Change(m) => m.entity_indices(),
            Self::Swap(m) => m.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Change(m) => m.variable_name(),
            Self::Swap(m) => m.variable_name(),
        }
    }
}
