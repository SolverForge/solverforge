solverforge::planning_model! {
    root = "crates/solverforge/tests/derive_macros/aliased_route_plan";

    mod aliased_route_plan;
    mod route;
    mod visit;

    pub use aliased_route_plan::AliasedRoutePlan;
    pub use route::Route;
    pub use route::Route as VehicleRoute;
    pub use visit::Visit;
}
