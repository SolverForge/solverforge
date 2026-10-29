solverforge::planning_model! {
    root = "crates/solverforge/tests/invalid_embedded_solver_toml/domain";

    mod plan;
    mod resource;
    mod task;

    pub use plan::Plan;
    pub use resource::Resource;
    pub use task::Task;
}
