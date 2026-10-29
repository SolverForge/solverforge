solverforge::planning_model! {
    root = "crates/solverforge/tests/readme_examples/staffing";

    mod assignment;
    mod employee;
    mod schedule;
    mod shift;

    pub use assignment::Assignment;
    pub use employee::Employee;
    pub use schedule::Schedule;
    pub use shift::Shift;
}
