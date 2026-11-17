solverforge::planning_model! {
    root = "crates/solverforge/tests/list_clarke_wright_fixed_fleet/domain";

    mod customer;
    mod fleet_plan;
    mod route;
    pub mod strict_savings_hooks;

    pub use customer::Customer;
    pub use fleet_plan::{build_fleet_plan, FleetPlan};
    pub use route::Route;
}
