solverforge::planning_model! {
    root = "crates/solverforge/tests/mixed_variable_order_runtime";

    mod plan;
    mod route;
    mod visit;

    pub use plan::Plan;
    pub use route::Route;
    pub use visit::Visit;
}
