solverforge::planning_model! {
    root = "crates/solverforge/tests/scalar_target_source_reuse/domain";

    mod plan;
    mod shift;

    pub use plan::Plan;
    pub use shift::Shift;
}
