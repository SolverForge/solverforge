// ChildPhases trait and tuple implementations.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::phase::Phase;
use crate::scope::SolverScope;

/// Trait for child phases that can solve a partition.
///
/// Implemented for tuples of phases via macro.
pub trait ChildPhases<S, D>
where
    S: PlanningSolution,
    D: Director<S>,
{
    // Runs all child phases on the solver scope.
    fn solve_all(&mut self, solver_scope: &mut SolverScope<S, D>);
}

// Implement ChildPhases for tuples using macro
macro_rules! impl_child_phases_tuple {
    ($($idx:tt: $P:ident),+) => {
        impl<S, D, $($P),+> ChildPhases<S, D> for ($($P,)+)
        where
            S: PlanningSolution,
            D: Director<S>,
            $($P: Phase<S, D>,)+
        {
            fn solve_all(&mut self, solver_scope: &mut SolverScope<S, D>) {
                $(
                    self.$idx.solve(solver_scope);
                )+
            }
        }
    };
}

impl_child_phases_tuple!(0: P0);
impl_child_phases_tuple!(0: P0, 1: P1);
impl_child_phases_tuple!(0: P0, 1: P1, 2: P2);
impl_child_phases_tuple!(0: P0, 1: P1, 2: P2, 3: P3);
impl_child_phases_tuple!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4);
impl_child_phases_tuple!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5);
impl_child_phases_tuple!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5, 6: P6);
impl_child_phases_tuple!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5, 6: P6, 7: P7);
