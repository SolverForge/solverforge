//! BasicVariableMove - unified move type for basic variable local search.
//!
//! Combines ChangeMove and SwapMove into a single enum for effective
//! neighborhood exploration. This enables the move selector to generate
//! both move types in a single pass.
//!
//! # Zero-Erasure Design
//!
//! This enum delegates to the underlying move types which use typed
//! function pointers. No `Arc<dyn>`, no `Box<dyn Any>`, no downcasting.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::traits::Move;
use super::{ChangeMove, SwapMove};

/// Unified move type for basic variable local search.
///
/// Combines ChangeMove (assign value to entity) and SwapMove (exchange
/// values between entities) for comprehensive neighborhood exploration.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The variable value type
pub enum BasicVariableMove<S, V> {
    /// Assigns a value to an entity's planning variable.
    Change(ChangeMove<S, V>),
    /// Swaps values between two entities.
    Swap(SwapMove<S, V>),
}

impl<S, V: Clone> Clone for BasicVariableMove<S, V> {
    fn clone(&self) -> Self {
        match self {
            Self::Change(m) => Self::Change(m.clone()),
            Self::Swap(m) => Self::Swap(*m),
        }
    }
}

impl<S, V: Copy> Copy for BasicVariableMove<S, V> {}

impl<S, V: Debug> Debug for BasicVariableMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Change(m) => m.fmt(f),
            Self::Swap(m) => m.fmt(f),
        }
    }
}

impl<S, V> Move<S> for BasicVariableMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<C>(&self, score_director: &ScoreDirector<S, C>) -> bool
    where
        C: solverforge_scoring::ConstraintSet<S, S::Score>,
    {
        match self {
            Self::Change(m) => m.is_doable(score_director),
            Self::Swap(m) => m.is_doable(score_director),
        }
    }

    fn do_move<C>(&self, score_director: &mut ScoreDirector<S, C>)
    where
        C: solverforge_scoring::ConstraintSet<S, S::Score>,
    {
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
