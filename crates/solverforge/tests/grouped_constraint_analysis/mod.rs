solverforge::planning_model! {
    root = "crates/solverforge/tests/grouped_constraint_analysis";

    mod nurse;
    mod roster;
    mod shift;

    pub use nurse::Nurse;
    pub use roster::{define_constraints, Roster};
    pub use shift::Shift;
}
