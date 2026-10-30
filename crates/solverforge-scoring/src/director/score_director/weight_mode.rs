//! Runtime constraint weight modes of the score director.
//!
//! The weight mode is a type parameter of `ScoreDirector`, chosen once when the
//! director is built. Every mode-dependent scoring call dispatches statically
//! through the sealed `Mode` methods below, so an `UnweightedMode` director
//! compiles to exactly the unweighted constraint-set calls: it holds no weight
//! storage, never binds multipliers, and never names a `*_weighted` method,
//! whatever the solution type. A `WeightedMode` director binds one multiplier
//! per constraint weight slot on construction and on every (re)initialization
//! and always takes the weighted calls.

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::api::analysis::ConstraintAnalysis;
use crate::api::constraint_set::{ConstraintResult, ConstraintSet};

/// How a [`ScoreDirector`](super::ScoreDirector) applies runtime constraint
/// weights. Sealed: the two modes are [`UnweightedMode`] and [`WeightedMode`].
pub trait WeightMode: sealed::Mode + Send + Sync + Sized + 'static {
    /// Whether this mode scales constraint contributions by bound weights.
    const WEIGHTED: bool;
}

/// Scores every constraint at its coded weight. Holds no weight storage and
/// ignores any weights the solution carries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnweightedMode;

/// Scales each constraint's contribution by the multiplier the working
/// solution's constraint weights assign it, rebound on every
/// (re)initialization.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WeightedMode {
    multipliers: Vec<u32>,
}

impl WeightMode for UnweightedMode {
    const WEIGHTED: bool = false;
}

impl WeightedMode {
    /// The bound multipliers, cut to exactly the set's slot count. Every
    /// composite set splits its slots at structural widths, so a slice whose
    /// length is the set's constant `constraint_count()` lets each nested split
    /// and per-slot load compile without a bounds check.
    #[inline(always)]
    fn slots<S, Sc, C>(&self, constraints: &C) -> &[u32]
    where
        Sc: Score,
        C: ConstraintSet<S, Sc>,
    {
        &self.multipliers[..constraints.constraint_count()]
    }
}

impl WeightMode for WeightedMode {
    const WEIGHTED: bool = true;
}

/// Whether `solution` needs a [`WeightedMode`] director: its type carries
/// constraint weights and some multiplier differs from `1`. Solves pick the
/// director's weight mode with this check, so a solution whose weights are
/// all `1` scores on the unweighted path.
pub fn requires_weighted_mode<S: PlanningSolution>(solution: &S) -> bool {
    S::HAS_CONSTRAINT_WEIGHTS
        && solution
            .constraint_weights()
            .is_some_and(|weights| weights.iter().any(|(_, multiplier)| multiplier != 1))
}

pub(crate) mod sealed {
    use super::*;

    /// The mode-dependent scoring calls. `UnweightedMode` forwards to the
    /// unweighted constraint-set methods verbatim; `WeightedMode` passes its
    /// bound multipliers to the weighted ones.
    pub trait Mode {
        fn unbound() -> Self;

        fn bind<S, C>(&mut self, solution: &S, constraints: &C)
        where
            S: PlanningSolution,
            C: ConstraintSet<S, S::Score>;

        fn initialize_all<S, Sc, C>(&self, constraints: &mut C, solution: &S) -> Sc
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>;

        fn evaluate_all<S, Sc, C>(&self, constraints: &C, solution: &S) -> Sc
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>;

        fn evaluate_each<'a, S, Sc, C>(
            &self,
            constraints: &'a C,
            solution: &S,
        ) -> Vec<ConstraintResult<'a, Sc>>
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>;

        fn evaluate_detailed<'a, S, Sc, C>(
            &self,
            constraints: &'a C,
            solution: &S,
        ) -> Vec<ConstraintAnalysis<'a, Sc>>
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>;

        fn on_insert_all<S, Sc, C>(
            &self,
            constraints: &mut C,
            solution: &S,
            entity_index: usize,
            descriptor_index: usize,
        ) -> Sc
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>;

        fn on_retract_all<S, Sc, C>(
            &self,
            constraints: &mut C,
            solution: &S,
            entity_index: usize,
            descriptor_index: usize,
        ) -> Sc
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>;
    }

    impl Mode for UnweightedMode {
        #[inline(always)]
        fn unbound() -> Self {
            Self
        }

        #[inline(always)]
        fn bind<S, C>(&mut self, _solution: &S, _constraints: &C)
        where
            S: PlanningSolution,
            C: ConstraintSet<S, S::Score>,
        {
        }

        #[inline(always)]
        fn initialize_all<S, Sc, C>(&self, constraints: &mut C, solution: &S) -> Sc
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>,
        {
            constraints.initialize_all(solution)
        }

        #[inline(always)]
        fn evaluate_all<S, Sc, C>(&self, constraints: &C, solution: &S) -> Sc
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>,
        {
            constraints.evaluate_all(solution)
        }

        #[inline(always)]
        fn evaluate_each<'a, S, Sc, C>(
            &self,
            constraints: &'a C,
            solution: &S,
        ) -> Vec<ConstraintResult<'a, Sc>>
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>,
        {
            constraints.evaluate_each(solution)
        }

        #[inline(always)]
        fn evaluate_detailed<'a, S, Sc, C>(
            &self,
            constraints: &'a C,
            solution: &S,
        ) -> Vec<ConstraintAnalysis<'a, Sc>>
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>,
        {
            constraints.evaluate_detailed(solution)
        }

        #[inline(always)]
        fn on_insert_all<S, Sc, C>(
            &self,
            constraints: &mut C,
            solution: &S,
            entity_index: usize,
            descriptor_index: usize,
        ) -> Sc
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>,
        {
            constraints.on_insert_all(solution, entity_index, descriptor_index)
        }

        #[inline(always)]
        fn on_retract_all<S, Sc, C>(
            &self,
            constraints: &mut C,
            solution: &S,
            entity_index: usize,
            descriptor_index: usize,
        ) -> Sc
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>,
        {
            constraints.on_retract_all(solution, entity_index, descriptor_index)
        }
    }

    impl Mode for WeightedMode {
        fn unbound() -> Self {
            Self::default()
        }

        /// Resolves the solution's multiplier for every weight slot by name.
        fn bind<S, C>(&mut self, solution: &S, constraints: &C)
        where
            S: PlanningSolution,
            C: ConstraintSet<S, S::Score>,
        {
            let mut refs = Vec::with_capacity(constraints.constraint_count());
            constraints.constraint_weight_refs(&mut refs);
            debug_assert_eq!(
                refs.len(),
                constraints.constraint_count(),
                "constraint set must expose one weight slot per constraint"
            );
            self.multipliers.clear();
            match solution.constraint_weights() {
                Some(weights) => self.multipliers.extend(
                    refs.iter()
                        .map(|constraint| weights.multiplier_for(constraint)),
                ),
                None => self.multipliers.resize(refs.len(), 1),
            }
        }

        fn initialize_all<S, Sc, C>(&self, constraints: &mut C, solution: &S) -> Sc
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>,
        {
            constraints.initialize_all_weighted(solution, self.slots(constraints))
        }

        fn evaluate_all<S, Sc, C>(&self, constraints: &C, solution: &S) -> Sc
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>,
        {
            constraints.evaluate_all_weighted(solution, self.slots(constraints))
        }

        fn evaluate_each<'a, S, Sc, C>(
            &self,
            constraints: &'a C,
            solution: &S,
        ) -> Vec<ConstraintResult<'a, Sc>>
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>,
        {
            constraints.evaluate_each_weighted(solution, &self.multipliers)
        }

        fn evaluate_detailed<'a, S, Sc, C>(
            &self,
            constraints: &'a C,
            solution: &S,
        ) -> Vec<ConstraintAnalysis<'a, Sc>>
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>,
        {
            constraints.evaluate_detailed_weighted(solution, &self.multipliers)
        }

        #[inline(always)]
        fn on_insert_all<S, Sc, C>(
            &self,
            constraints: &mut C,
            solution: &S,
            entity_index: usize,
            descriptor_index: usize,
        ) -> Sc
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>,
        {
            constraints.on_insert_all_weighted(
                solution,
                entity_index,
                descriptor_index,
                self.slots(constraints),
            )
        }

        #[inline(always)]
        fn on_retract_all<S, Sc, C>(
            &self,
            constraints: &mut C,
            solution: &S,
            entity_index: usize,
            descriptor_index: usize,
        ) -> Sc
        where
            Sc: Score,
            C: ConstraintSet<S, Sc>,
        {
            constraints.on_retract_all_weighted(
                solution,
                entity_index,
                descriptor_index,
                self.slots(constraints),
            )
        }
    }
}

/// Entity-row deltas as the list-shadow bookkeeping applies them, through the
/// director's weight mode.
pub(super) trait RowDeltas<S, Sc> {
    fn insert_row(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;
    fn retract_row(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;
}

pub(super) struct ModeRows<'a, C, M> {
    pub(super) constraints: &'a mut C,
    pub(super) mode: &'a M,
}

impl<S, Sc, C, M> RowDeltas<S, Sc> for ModeRows<'_, C, M>
where
    Sc: Score,
    C: ConstraintSet<S, Sc>,
    M: WeightMode,
{
    #[inline(always)]
    fn insert_row(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.mode
            .on_insert_all(self.constraints, solution, entity_index, descriptor_index)
    }

    #[inline(always)]
    fn retract_row(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.mode
            .on_retract_all(self.constraints, solution, entity_index, descriptor_index)
    }
}
