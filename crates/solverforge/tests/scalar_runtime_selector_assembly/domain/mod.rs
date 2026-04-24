solverforge::planning_model! {
    root = "crates/solverforge/tests/scalar_runtime_selector_assembly/domain";

    mod plan;
    mod resource;
    mod task;

    pub use plan::Plan;
    pub use resource::Resource;
    pub use task::Task;
}
