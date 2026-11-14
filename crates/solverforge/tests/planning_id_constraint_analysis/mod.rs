solverforge::planning_model! {
    root = "crates/solverforge/tests/planning_id_constraint_analysis";

    mod nurse;
    mod roster;
    mod shift;
    mod ward;

    pub use nurse::Nurse;
    pub use roster::{define_constraints, Roster};
    pub use shift::{Shift, ShiftId};
    pub use ward::Ward;
}
