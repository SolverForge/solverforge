//! Monomorphic forager enum for construction heuristics.
//!
//! All construction forager types wrapped in a single enum for config-driven
//! selection without type erasure.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_config::ConstructionHeuristicType;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;

use super::forager::{
    BestFitForager, CheapestInsertionForager, ConstructionForager, FirstFeasibleForager,
    FirstFitForager, RegretInsertionForager, StrongestFitForager, WeakestFitForager,
};
use super::Placement;

/// Monomorphic construction forager enum - runtime selection without type erasure.
///
/// Wraps all construction forager types, enabling config-driven selection
/// while preserving concrete types throughout the solver pipeline.
pub enum ConstructionForagerImpl<S, M> {
    /// First fit - accepts first doable move.
    FirstFit(FirstFitForager<S, M>),
    /// Best fit - evaluates all, picks best score.
    BestFit(BestFitForager<S, M>),
    /// First feasible - first move with feasible score.
    FirstFeasible(FirstFeasibleForager<S, M>),
    /// Weakest fit - picks move with lowest strength.
    WeakestFit(WeakestFitForager<S, M>),
    /// Strongest fit - picks move with highest strength.
    StrongestFit(StrongestFitForager<S, M>),
    /// Cheapest insertion - picks move with minimum insertion cost.
    CheapestInsertion(CheapestInsertionForager<S, M>),
    /// Regret insertion - picks move with maximum regret.
    RegretInsertion(RegretInsertionForager<S, M>),
}

impl<S, M> ConstructionForagerImpl<S, M> {
    /// Creates a first fit forager.
    pub fn first_fit() -> Self {
        Self::FirstFit(FirstFitForager::new())
    }

    /// Creates a best fit forager.
    pub fn best_fit() -> Self {
        Self::BestFit(BestFitForager::new())
    }

    /// Creates a first feasible forager.
    pub fn first_feasible() -> Self {
        Self::FirstFeasible(FirstFeasibleForager::new())
    }

    /// Creates a weakest fit forager with custom strength function.
    pub fn weakest_fit(strength_fn: fn(&M) -> i64) -> Self {
        Self::WeakestFit(WeakestFitForager::new(strength_fn))
    }

    /// Creates a strongest fit forager with custom strength function.
    pub fn strongest_fit(strength_fn: fn(&M) -> i64) -> Self {
        Self::StrongestFit(StrongestFitForager::new(strength_fn))
    }

    /// Creates a cheapest insertion forager.
    pub fn cheapest_insertion() -> Self {
        Self::CheapestInsertion(CheapestInsertionForager::new())
    }

    /// Creates a regret insertion forager with the given k value.
    pub fn regret_insertion(k: usize) -> Self {
        Self::RegretInsertion(RegretInsertionForager::new(k))
    }

    /// Creates a forager from configuration type.
    ///
    /// For WeakestFit/StrongestFit, uses BestFit as fallback since strength
    /// functions require domain-specific knowledge. Use `weakest_fit()` or
    /// `strongest_fit()` directly when you have a strength function.
    pub fn from_config(config: ConstructionHeuristicType) -> Self {
        match config {
            ConstructionHeuristicType::FirstFit => Self::first_fit(),
            ConstructionHeuristicType::FirstFitDecreasing => Self::first_fit(),
            ConstructionHeuristicType::WeakestFit => {
                // WeakestFit requires domain-specific strength function.
                // Fall back to BestFit which works universally.
                Self::best_fit()
            }
            ConstructionHeuristicType::WeakestFitDecreasing => Self::best_fit(),
            ConstructionHeuristicType::StrongestFit => {
                // StrongestFit requires domain-specific strength function.
                // Fall back to BestFit which works universally.
                Self::best_fit()
            }
            ConstructionHeuristicType::StrongestFitDecreasing => Self::best_fit(),
            ConstructionHeuristicType::CheapestInsertion => Self::cheapest_insertion(),
            ConstructionHeuristicType::AllocateEntityFromQueue => Self::first_fit(),
            ConstructionHeuristicType::AllocateToValueFromQueue => Self::first_fit(),
        }
    }

    /// Creates a forager from configuration with custom strength function.
    ///
    /// Use this when you have a domain-specific strength function for
    /// WeakestFit/StrongestFit variants.
    pub fn from_config_with_strength(
        config: ConstructionHeuristicType,
        strength_fn: fn(&M) -> i64,
    ) -> Self {
        match config {
            ConstructionHeuristicType::FirstFit => Self::first_fit(),
            ConstructionHeuristicType::FirstFitDecreasing => Self::first_fit(),
            ConstructionHeuristicType::WeakestFit => Self::weakest_fit(strength_fn),
            ConstructionHeuristicType::WeakestFitDecreasing => Self::weakest_fit(strength_fn),
            ConstructionHeuristicType::StrongestFit => Self::strongest_fit(strength_fn),
            ConstructionHeuristicType::StrongestFitDecreasing => Self::strongest_fit(strength_fn),
            ConstructionHeuristicType::CheapestInsertion => Self::cheapest_insertion(),
            ConstructionHeuristicType::AllocateEntityFromQueue => Self::first_fit(),
            ConstructionHeuristicType::AllocateToValueFromQueue => Self::first_fit(),
        }
    }
}

impl<S, M> Default for ConstructionForagerImpl<S, M> {
    fn default() -> Self {
        Self::first_fit()
    }
}

impl<S, M> Clone for ConstructionForagerImpl<S, M> {
    fn clone(&self) -> Self {
        match self {
            Self::FirstFit(f) => Self::FirstFit(*f),
            Self::BestFit(f) => Self::BestFit(*f),
            Self::FirstFeasible(f) => Self::FirstFeasible(*f),
            Self::WeakestFit(f) => Self::WeakestFit(*f),
            Self::StrongestFit(f) => Self::StrongestFit(*f),
            Self::CheapestInsertion(f) => Self::CheapestInsertion(*f),
            Self::RegretInsertion(f) => Self::RegretInsertion(*f),
        }
    }
}

impl<S, M> Copy for ConstructionForagerImpl<S, M> {}

impl<S, M> Debug for ConstructionForagerImpl<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FirstFit(fo) => fo.fmt(f),
            Self::BestFit(fo) => fo.fmt(f),
            Self::FirstFeasible(fo) => fo.fmt(f),
            Self::WeakestFit(fo) => fo.fmt(f),
            Self::StrongestFit(fo) => fo.fmt(f),
            Self::CheapestInsertion(fo) => fo.fmt(f),
            Self::RegretInsertion(fo) => fo.fmt(f),
        }
    }
}

impl<S, M> ConstructionForager<S, M> for ConstructionForagerImpl<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move_index<D: ScoreDirector<S>>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut D,
    ) -> Option<usize> {
        match self {
            Self::FirstFit(fo) => fo.pick_move_index(placement, score_director),
            Self::BestFit(fo) => fo.pick_move_index(placement, score_director),
            Self::FirstFeasible(fo) => fo.pick_move_index(placement, score_director),
            Self::WeakestFit(fo) => fo.pick_move_index(placement, score_director),
            Self::StrongestFit(fo) => fo.pick_move_index(placement, score_director),
            Self::CheapestInsertion(fo) => fo.pick_move_index(placement, score_director),
            Self::RegretInsertion(fo) => fo.pick_move_index(placement, score_director),
        }
    }
}
