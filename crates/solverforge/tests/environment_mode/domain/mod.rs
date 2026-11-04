solverforge::planning_model! {
    root = "crates/solverforge/tests/environment_mode/domain";

    mod plan;
    mod resource;
    mod task;

    pub use plan::Plan;
    pub use resource::Resource;
    pub use task::Task;
}
