/* Constraint definitions.

   Each constraint is a separate function returning an `IncrementalConstraint`.
   Assemble them into a tuple — the solver scores the solution against all of them.

   Add as many constraints as your problem needs. The tuple supports up to 12 elements;
   use nested tuples for more. */

mod all_assigned;

pub use self::assemble::create_constraints;

mod assemble {
    use super::*;
    use crate::domain::Plan;
    use solverforge::prelude::*;

    pub fn create_constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
        (all_assigned::constraint(),)
    }
}
