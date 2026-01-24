//! Composite termination conditions (AND/OR).
//!
//! Uses macro-generated tuple implementations for zero-erasure architecture.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;

use super::Termination;
use crate::scope::SolverScope;

/// Combines multiple terminations with OR logic.
///
/// Terminates when ANY of the child terminations triggers.
///
/// # Example
///
/// ```
/// use solverforge_solver::termination::{OrTermination, TimeTermination, StepCountTermination};
/// use solverforge_scoring::api::constraint_set::ConstraintSet;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
/// use std::time::Duration;
///
/// #[derive(Clone)]
/// struct MySolution;
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { None }
///     fn set_score(&mut self, _: Option<Self::Score>) {}
/// }
///
/// // Terminate after 30 seconds OR 1000 steps
/// let term: OrTermination<_, MySolution, ()> = OrTermination::new((
///     TimeTermination::seconds(30),
///     StepCountTermination::new(1000),
/// ));
/// ```
#[derive(Clone)]
pub struct OrTermination<T, S, C>(pub T, PhantomData<fn(S, C)>);

impl<T: Debug, S, C> Debug for OrTermination<T, S, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OrTermination").field(&self.0).finish()
    }
}

impl<T, S, C> OrTermination<T, S, C> {
    pub fn new(terminations: T) -> Self {
        Self(terminations, PhantomData)
    }
}

/// Combines multiple terminations with AND logic.
///
/// All terminations must agree before solving terminates.
///
/// # Example
///
/// ```
/// use solverforge_solver::termination::{AndTermination, TimeTermination, StepCountTermination};
/// use solverforge_scoring::api::constraint_set::ConstraintSet;
/// use solverforge_core::score::SimpleScore;
/// use solverforge_core::domain::PlanningSolution;
/// use std::time::Duration;
///
/// #[derive(Clone)]
/// struct MySolution;
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { None }
///     fn set_score(&mut self, _: Option<Self::Score>) {}
/// }
///
/// // Terminate when both conditions are met
/// let term: AndTermination<_, MySolution, ()> = AndTermination::new((
///     TimeTermination::seconds(10),
///     StepCountTermination::new(100),
/// ));
/// ```
#[derive(Clone)]
pub struct AndTermination<T, S, C>(pub T, PhantomData<fn(S, C)>);

impl<T: Debug, S, C> Debug for AndTermination<T, S, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AndTermination").field(&self.0).finish()
    }
}

impl<T, S, C> AndTermination<T, S, C> {
    pub fn new(terminations: T) -> Self {
        Self(terminations, PhantomData)
    }
}

macro_rules! impl_composite_termination {
    ($($idx:tt: $T:ident),+) => {
        impl<S, C, $($T),+> Termination<S, C> for OrTermination<($($T,)+), S, C>
        where
            S: PlanningSolution,
            S::Score: Score,
            C: ConstraintSet<S, S::Score>,
            $($T: Termination<S, C>,)+
        {
            fn is_terminated(&self, solver_scope: &SolverScope<S, C>) -> bool {
                $(
                    if self.0.$idx.is_terminated(solver_scope) {
                        return true;
                    }
                )+
                false
            }
        }

        impl<S, C, $($T),+> Termination<S, C> for AndTermination<($($T,)+), S, C>
        where
            S: PlanningSolution,
            S::Score: Score,
            C: ConstraintSet<S, S::Score>,
            $($T: Termination<S, C>,)+
        {
            fn is_terminated(&self, solver_scope: &SolverScope<S, C>) -> bool {
                $(
                    if !self.0.$idx.is_terminated(solver_scope) {
                        return false;
                    }
                )+
                true
            }
        }
    };
}

impl_composite_termination!(0: T0);
impl_composite_termination!(0: T0, 1: T1);
impl_composite_termination!(0: T0, 1: T1, 2: T2);
impl_composite_termination!(0: T0, 1: T1, 2: T2, 3: T3);
impl_composite_termination!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4);
impl_composite_termination!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5);
impl_composite_termination!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6);
impl_composite_termination!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7);
