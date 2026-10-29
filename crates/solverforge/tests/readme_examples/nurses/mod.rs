solverforge::planning_model! {
    root = "crates/solverforge/tests/readme_examples/nurses";

    mod nurse;
    mod schedule;
    mod shift;

    pub use nurse::Nurse;
    pub use schedule::Schedule;
    pub use shift::Shift;
}
