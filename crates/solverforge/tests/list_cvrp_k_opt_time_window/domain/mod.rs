solverforge::planning_model! {
    root = "crates/solverforge/tests/list_cvrp_k_opt_time_window/domain";

    mod plan;
    mod route;

    pub use plan::{build_plan, Plan};
    pub use route::Route;
}
