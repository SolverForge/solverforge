//! Solver phases for different solving strategies.
//!
//! # Phase Composition
//!
//! Phases can be composed using [`PhaseSequence`], which wraps a tuple of phases
//! and runs them in order. This uses macro-generated tuple implementations
//! for zero type erasure.
//!
//! ```
//! use solverforge_solver::phase::PhaseSequence;
//! // PhaseSequence((construction_phase, local_search_phase))
//! ```

pub mod construction;
pub mod exhaustive;
pub mod localsearch;
pub mod partitioned;
pub mod vnd;

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::scope::SolverScope;

/// A phase of the solving process.
///
/// Generic over `D: ScoreDirector<S>` for zero type erasure.
pub trait Phase<S: PlanningSolution, D: ScoreDirector<S>>: Send + Debug {
    /// Executes this phase.
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>);

    /// Returns the name of this phase type.
    fn phase_type_name(&self) -> &'static str;
}

/// A composite phase that runs phases in sequence.
///
/// Wraps a tuple of phases and executes them in order, stopping early
/// if termination is requested. Uses macro-generated tuple implementations
/// for zero type erasure.
///
/// # Examples
///
/// ```ignore
/// // Two-phase sequence (construction + local search)
/// let phases = PhaseSequence((construction_phase, local_search_phase));
///
/// // Three-phase sequence
/// let phases = PhaseSequence((phase1, phase2, phase3));
/// ```
#[derive(Debug)]
pub struct PhaseSequence<T>(pub T);

impl<T> PhaseSequence<T> {
    /// Creates a new phase sequence from a tuple of phases.
    pub fn new(phases: T) -> Self {
        Self(phases)
    }
}

/// Generates `Phase` implementations for tuples of phases.
macro_rules! impl_phase_sequence {
    // Base case: single phase
    ($idx:tt: $P:ident) => {
        impl<S, D, $P> Phase<S, D> for PhaseSequence<($P,)>
        where
            S: PlanningSolution,
            D: ScoreDirector<S>,
            $P: Phase<S, D>,
        {
            fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
                (self.0).$idx.solve(solver_scope);
            }

            fn phase_type_name(&self) -> &'static str {
                "PhaseSequence"
            }
        }
    };

    // Recursive case: multiple phases
    ($($idx:tt: $P:ident),+) => {
        impl<S, D, $($P),+> Phase<S, D> for PhaseSequence<($($P,)+)>
        where
            S: PlanningSolution,
            D: ScoreDirector<S>,
            $($P: Phase<S, D>,)+
        {
            fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
                impl_phase_sequence!(@solve self, solver_scope, $($idx: $P),+);
            }

            fn phase_type_name(&self) -> &'static str {
                "PhaseSequence"
            }
        }
    };

    // Helper: generate solve calls with early termination checks
    (@solve $self:ident, $scope:ident, $first_idx:tt: $first_P:ident $(, $idx:tt: $P:ident)*) => {
        ($self.0).$first_idx.solve($scope);
        $(
            if !$scope.is_terminate_early() {
                ($self.0).$idx.solve($scope);
            }
        )*
    };
}

// Implement for tuples of size 1 through 8
impl_phase_sequence!(0: P0);
impl_phase_sequence!(0: P0, 1: P1);
impl_phase_sequence!(0: P0, 1: P1, 2: P2);
impl_phase_sequence!(0: P0, 1: P1, 2: P2, 3: P3);
impl_phase_sequence!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4);
impl_phase_sequence!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5);
impl_phase_sequence!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5, 6: P6);
impl_phase_sequence!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5, 6: P6, 7: P7);

