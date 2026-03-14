/* Constraint definitions for Employee Scheduling.

   Each constraint is a separate function returning an `IncrementalConstraint`.
   The tuple assembles them into a `ConstraintSet` for the solver. */

mod balance;
mod no_overlap;
mod one_per_day;
mod required_skill;
mod ten_hour_gap;
mod unavailable;
mod undesired_desired;

pub use self::assemble::create_constraints;

mod assemble {
    use super::*;
    use crate::domain::EmployeeSchedule;
    use solverforge::prelude::*;

    pub fn create_constraints() -> impl ConstraintSet<EmployeeSchedule, HardSoftDecimalScore> {
        (
            required_skill::constraint(),
            no_overlap::constraint(),
            ten_hour_gap::constraint(),
            one_per_day::constraint(),
            unavailable::constraint(),
            undesired_desired::undesired_constraint(),
            undesired_desired::desired_constraint(),
            balance::constraint(),
        )
    }
}
