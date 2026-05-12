solverforge::planning_model! {
    root = "crates/solverforge/tests/constraint_accessors/domain";

    mod employee;
    mod schedule;
    mod shift;

    pub use employee::Employee;
    pub use schedule::{Schedule, ScheduleConstraintStreams};
    pub use shift::Shift;
}
