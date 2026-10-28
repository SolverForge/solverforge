solverforge::planning_model! {
    root = "crates/solverforge/tests/realtime_list_problem_change/domain";

    mod plan;
    mod route;
    mod visit;

    pub use plan::RoutePlan;
    pub use route::Route;
    pub use visit::Visit;
}
