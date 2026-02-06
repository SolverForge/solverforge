//! Either move — monomorphized union of DynamicChangeMove and DynamicSwapMove.

use std::fmt;

use solverforge_scoring::ScoreDirector;
use solverforge_solver::heuristic::r#move::Move;

use super::change_move::DynamicChangeMove;
use super::swap_move::DynamicSwapMove;
use crate::solution::DynamicSolution;

/// A move that is either a change or a swap.
#[derive(Clone)]
pub enum DynamicEitherMove {
    Change(DynamicChangeMove),
    Swap(DynamicSwapMove),
}

impl fmt::Debug for DynamicEitherMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Change(m) => m.fmt(f),
            Self::Swap(m) => m.fmt(f),
        }
    }
}

impl Move<DynamicSolution> for DynamicEitherMove {
    fn is_doable<D: ScoreDirector<DynamicSolution>>(&self, score_director: &D) -> bool {
        match self {
            Self::Change(m) => m.is_doable(score_director),
            Self::Swap(m) => m.is_doable(score_director),
        }
    }

    fn do_move<D: ScoreDirector<DynamicSolution>>(&self, score_director: &mut D) {
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
