//! Composite termination conditions (AND/OR).
//!
//! Uses macro-generated tuple implementations for zero-erasure architecture.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

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
/// use solverforge_scoring::SimpleScoreDirector;
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
/// type MyDirector = SimpleScoreDirector<MySolution, fn(&MySolution) -> SimpleScore>;
///
/// // Terminate after 30 seconds OR 1000 steps
/// let term: OrTermination<_, MySolution, MyDirector> = OrTermination::new((
///     TimeTermination::seconds(30),
///     StepCountTermination::new(1000),
/// ));
/// ```
#[derive(Clone)]
pub struct OrTermination<T, S, D>(pub T, PhantomData<fn(S, D)>);

impl<T: Debug, S, D> Debug for OrTermination<T, S, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OrTermination").field(&self.0).finish()
    }
}

impl<T, S, D> OrTermination<T, S, D> {
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
/// use solverforge_scoring::SimpleScoreDirector;
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
/// type MyDirector = SimpleScoreDirector<MySolution, fn(&MySolution) -> SimpleScore>;
///
/// // Terminate when both conditions are met
/// let term: AndTermination<_, MySolution, MyDirector> = AndTermination::new((
///     TimeTermination::seconds(10),
///     StepCountTermination::new(100),
/// ));
/// ```
#[derive(Clone)]
pub struct AndTermination<T, S, D>(pub T, PhantomData<fn(S, D)>);

impl<T: Debug, S, D> Debug for AndTermination<T, S, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AndTermination").field(&self.0).finish()
    }
}

impl<T, S, D> AndTermination<T, S, D> {
    pub fn new(terminations: T) -> Self {
        Self(terminations, PhantomData)
    }
}

macro_rules! impl_composite_termination {
    ($($idx:tt: $T:ident),+) => {
        impl<S, D, $($T),+> Termination<S, D> for OrTermination<($($T,)+), S, D>
        where
            S: PlanningSolution,
            D: ScoreDirector<S>,
            $($T: Termination<S, D>,)+
        {
            fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
                $(
                    if self.0.$idx.is_terminated(solver_scope) {
                        return true;
                    }
                )+
                false
            }

            fn install_inphase_limits(&self, solver_scope: &mut SolverScope<S, D>) {
                // Propagate in-phase limits from all child terminations.
                // For OR, each child independently may set a limit.
                $(
                    self.0.$idx.install_inphase_limits(solver_scope);
                )+
            }
        }

        impl<S, D, $($T),+> Termination<S, D> for AndTermination<($($T,)+), S, D>
        where
            S: PlanningSolution,
            D: ScoreDirector<S>,
            $($T: Termination<S, D>,)+
        {
            fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
                $(
                    if !self.0.$idx.is_terminated(solver_scope) {
                        return false;
                    }
                )+
                true
            }

            fn install_inphase_limits(&self, solver_scope: &mut SolverScope<S, D>) {
                $(
                    self.0.$idx.install_inphase_limits(solver_scope);
                )+
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
