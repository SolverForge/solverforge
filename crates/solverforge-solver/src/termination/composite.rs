//! Composite termination conditions (AND/OR).
//!
//! Uses macro-generated tuple implementations for zero type erasure.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Termination;
use crate::scope::SolverScope;

/// Combines multiple terminations with OR logic (any must terminate).
///
/// Wraps a tuple of terminations. Terminates when ANY child terminates.
///
/// # Examples
///
/// ```ignore
/// // Terminate after 30 seconds OR 1000 steps
/// let termination = OrTermination((
///     TimeTermination::new(Duration::from_secs(30)),
///     StepCountTermination::new(1000),
/// ));
/// ```
#[derive(Debug)]
pub struct OrTermination<T>(pub T);

impl<T> OrTermination<T> {
    /// Creates a new OR termination from a tuple of terminations.
    pub fn new(terminations: T) -> Self {
        Self(terminations)
    }
}

/// Generates `Termination` implementations for OR tuples.
macro_rules! impl_or_termination {
    // Single termination
    ($idx:tt: $T:ident) => {
        impl<S, D, $T> Termination<S, D> for OrTermination<($T,)>
        where
            S: PlanningSolution,
            D: ScoreDirector<S>,
            $T: Termination<S, D>,
        {
            fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
                (self.0).$idx.is_terminated(solver_scope)
            }
        }
    };

    // Multiple terminations - any must be true
    ($($idx:tt: $T:ident),+) => {
        impl<S, D, $($T),+> Termination<S, D> for OrTermination<($($T,)+)>
        where
            S: PlanningSolution,
            D: ScoreDirector<S>,
            $($T: Termination<S, D>,)+
        {
            fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
                $((self.0).$idx.is_terminated(solver_scope))||+
            }
        }
    };
}

impl_or_termination!(0: T0);
impl_or_termination!(0: T0, 1: T1);
impl_or_termination!(0: T0, 1: T1, 2: T2);
impl_or_termination!(0: T0, 1: T1, 2: T2, 3: T3);
impl_or_termination!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4);
impl_or_termination!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5);
impl_or_termination!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6);
impl_or_termination!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7);

/// Combines multiple terminations with AND logic (all must terminate).
///
/// Wraps a tuple of terminations. Terminates when ALL children terminate.
///
/// # Examples
///
/// ```ignore
/// // Terminate only when BOTH score is feasible AND 100 steps passed
/// let termination = AndTermination((
///     BestScoreFeasibleTermination::new(),
///     StepCountTermination::new(100),
/// ));
/// ```
#[derive(Debug)]
pub struct AndTermination<T>(pub T);

impl<T> AndTermination<T> {
    /// Creates a new AND termination from a tuple of terminations.
    pub fn new(terminations: T) -> Self {
        Self(terminations)
    }
}

/// Generates `Termination` implementations for AND tuples.
macro_rules! impl_and_termination {
    // Single termination
    ($idx:tt: $T:ident) => {
        impl<S, D, $T> Termination<S, D> for AndTermination<($T,)>
        where
            S: PlanningSolution,
            D: ScoreDirector<S>,
            $T: Termination<S, D>,
        {
            fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
                (self.0).$idx.is_terminated(solver_scope)
            }
        }
    };

    // Multiple terminations - all must be true
    ($($idx:tt: $T:ident),+) => {
        impl<S, D, $($T),+> Termination<S, D> for AndTermination<($($T,)+)>
        where
            S: PlanningSolution,
            D: ScoreDirector<S>,
            $($T: Termination<S, D>,)+
        {
            fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
                $((self.0).$idx.is_terminated(solver_scope))&&+
            }
        }
    };
}

impl_and_termination!(0: T0);
impl_and_termination!(0: T0, 1: T1);
impl_and_termination!(0: T0, 1: T1, 2: T2);
impl_and_termination!(0: T0, 1: T1, 2: T2, 3: T3);
impl_and_termination!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4);
impl_and_termination!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5);
impl_and_termination!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6);
impl_and_termination!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7);
