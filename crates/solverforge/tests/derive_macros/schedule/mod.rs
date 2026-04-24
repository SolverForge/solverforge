solverforge::planning_model! {
    root = "crates/solverforge/tests/derive_macros/schedule";

    mod employee;
    mod schedule;
    mod shift;

    pub use employee::Employee;
    pub use schedule::Schedule;
    pub use shift::Shift;
}
