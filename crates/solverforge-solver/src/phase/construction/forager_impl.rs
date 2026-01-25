//! Monomorphic enum for construction foragers.
//!
//! Provides zero-erasure dispatch over all construction forager variants.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_config::ConstructionHeuristicType;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::ScoreDirector;

use super::forager::{
    BestFitForager, CheapestInsertionForager, ConstructionForager, FirstFeasibleForager,
    FirstFitForager, RegretInsertionForager,
};
use super::Placement;
use crate::heuristic::r#move::Move;

/// Monomorphic enum wrapping all construction forager implementations.
///
/// This enum enables config-driven forager selection without trait objects.
/// The `from_config` method converts parsed configuration to the appropriate variant.
pub enum ConstructionForagerImpl<S: PlanningSolution, M> {
    FirstFit(FirstFitForager<S, M>),
    BestFit(BestFitForager<S, M>),
    FirstFeasible(FirstFeasibleForager<S, M>),
    WeakestFit(PhantomData<fn() -> (S, M)>),
    StrongestFit(PhantomData<fn() -> (S, M)>),
    CheapestInsertion(CheapestInsertionForager<S, M>),
    RegretInsertion(RegretInsertionForager<S, M>),
}

impl<S: PlanningSolution, M> ConstructionForagerImpl<S, M> {
    /// Creates a forager from configuration.
    ///
    /// Maps each `ConstructionHeuristicType` to its corresponding forager.
    pub fn from_config(config: ConstructionHeuristicType) -> Self {
        match config {
            ConstructionHeuristicType::FirstFit => {
                ConstructionForagerImpl::FirstFit(FirstFitForager::new())
            }
            ConstructionHeuristicType::FirstFitDecreasing => {
                ConstructionForagerImpl::BestFit(BestFitForager::new())
            }
            ConstructionHeuristicType::WeakestFit
            | ConstructionHeuristicType::WeakestFitDecreasing => {
                ConstructionForagerImpl::WeakestFit(PhantomData)
            }
            ConstructionHeuristicType::StrongestFit
            | ConstructionHeuristicType::StrongestFitDecreasing => {
                ConstructionForagerImpl::StrongestFit(PhantomData)
            }
            ConstructionHeuristicType::CheapestInsertion => {
                ConstructionForagerImpl::CheapestInsertion(CheapestInsertionForager::new())
            }
            ConstructionHeuristicType::RegretInsertion => {
                ConstructionForagerImpl::RegretInsertion(RegretInsertionForager::new())
            }
            ConstructionHeuristicType::AllocateEntityFromQueue
            | ConstructionHeuristicType::AllocateToValueFromQueue => {
                ConstructionForagerImpl::FirstFeasible(FirstFeasibleForager::new())
            }
        }
    }
}

impl<S: PlanningSolution, M> Clone for ConstructionForagerImpl<S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: PlanningSolution, M> Copy for ConstructionForagerImpl<S, M> {}

impl<S: PlanningSolution, M> Debug for ConstructionForagerImpl<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FirstFit(_) => write!(f, "FirstFit"),
            Self::BestFit(_) => write!(f, "BestFit"),
            Self::FirstFeasible(_) => write!(f, "FirstFeasible"),
            Self::WeakestFit(_) => write!(f, "WeakestFit"),
            Self::StrongestFit(_) => write!(f, "StrongestFit"),
            Self::CheapestInsertion(_) => write!(f, "CheapestInsertion"),
            Self::RegretInsertion(_) => write!(f, "RegretInsertion"),
        }
    }
}

impl<S, M> ConstructionForager<S, M> for ConstructionForagerImpl<S, M>
where
    S: PlanningSolution + solverforge_scoring::ShadowVariableSupport,
    M: Move<S>,
{
    fn pick_move_index<C>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut ScoreDirector<S, C>,
    ) -> Option<usize>
    where
        C: ConstraintSet<S, S::Score>,
        S::Score: Score,
    {
        match self {
            Self::FirstFit(f) => f.pick_move_index(placement, score_director),
            Self::BestFit(f) => f.pick_move_index(placement, score_director),
            Self::FirstFeasible(f) => f.pick_move_index(placement, score_director),
            Self::CheapestInsertion(f) => f.pick_move_index(placement, score_director),
            Self::RegretInsertion(f) => f.pick_move_index(placement, score_director),
            Self::WeakestFit(_) => {
                let mut best_idx = None;
                let mut min_strength = None;
                for (idx, m) in placement.moves.iter().enumerate() {
                    if !m.is_doable(score_director) {
                        continue;
                    }
                    let strength = m.strength();
                    if min_strength.is_none() || strength < min_strength.unwrap() {
                        best_idx = Some(idx);
                        min_strength = Some(strength);
                    }
                }
                best_idx
            }
            Self::StrongestFit(_) => {
                let mut best_idx = None;
                let mut max_strength = None;
                for (idx, m) in placement.moves.iter().enumerate() {
                    if !m.is_doable(score_director) {
                        continue;
                    }
                    let strength = m.strength();
                    if max_strength.is_none() || strength > max_strength.unwrap() {
                        best_idx = Some(idx);
                        max_strength = Some(strength);
                    }
                }
                best_idx
            }
        }
    }
}
