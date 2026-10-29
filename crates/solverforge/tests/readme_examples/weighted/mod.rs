solverforge::planning_model! {
    root = "crates/solverforge/tests/readme_examples/weighted";

    mod schedule;
    mod shift;

    pub use schedule::Schedule;
    pub use shift::Shift;
}
