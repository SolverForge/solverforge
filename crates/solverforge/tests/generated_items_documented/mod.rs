solverforge::planning_model! {
    root = "crates/solverforge/tests/generated_items_documented";

    mod plan;
    mod route;
    mod visit;

    pub use plan::Plan;
    pub use plan::PlanConstraintStreams;
    pub use route::Route;
    pub use visit::Visit;
}
