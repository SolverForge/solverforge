solverforge::planning_model! {
    root = "crates/solverforge/tests/parallel_list_local_search/domain";

    mod plan;
    mod route;
    mod visit;

    pub use plan::RoutePlan;
    pub use route::Route;
    pub use visit::Visit;
}
