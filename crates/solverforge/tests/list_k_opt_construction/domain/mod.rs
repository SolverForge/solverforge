solverforge::planning_model! {
    root = "crates/solverforge/tests/list_k_opt_construction/domain";

    mod plan;
    mod route;

    pub use plan::sorting_plan;
    pub use route::Route;
}
