solverforge::planning_model! {
    root = "crates/solverforge/tests/model_sources/domain";

    mod employee;
    mod plan;
    mod shift;
    mod visit;

    pub use employee::Employee;
    pub use plan::Plan;
    pub use shift::Shift;
    pub use visit::Visit;
}
