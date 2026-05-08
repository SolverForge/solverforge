solverforge::planning_model! {
    root = "examples/minimal-shift-scheduling/src/domain";

    mod nurse;
    mod schedule;
    mod shift;

    pub use nurse::Nurse;
    pub use schedule::Schedule;
    pub use shift::Shift;
}
