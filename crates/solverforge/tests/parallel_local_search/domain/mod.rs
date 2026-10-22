solverforge::planning_model! {
    root = "crates/solverforge/tests/parallel_local_search/domain";

    mod plan;
    mod resource;
    mod task;

    pub use plan::Plan;
    pub use resource::Resource;
    pub use task::Task;
}
