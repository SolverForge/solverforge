solverforge::planning_model! {
    root = "crates/solverforge/tests/derive_macros/route_plan";

    mod route;
    mod route_plan;
    mod visit;

    pub use route::Route;
    pub use route_plan::RoutePlan;
    pub use visit::Visit;
}
