solverforge::planning_model! {
    root = "crates/solverforge/tests/derive_macros/shadow_plan";

    mod multi_owner_shadow_plan;
    mod routed_visit;
    mod shadow_route;
    mod shadow_shift;
    mod shift_visit;

    pub use multi_owner_shadow_plan::MultiOwnerShadowPlan;
    pub use routed_visit::RoutedVisit;
    pub use shadow_route::ShadowRoute;
    pub use shadow_shift::ShadowShift;
    pub use shift_visit::ShiftVisit;
}
