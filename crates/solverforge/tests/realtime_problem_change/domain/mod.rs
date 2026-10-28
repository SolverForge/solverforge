solverforge::planning_model! {
    root = "crates/solverforge/tests/realtime_problem_change/domain";

    mod plan;
    mod resource;
    mod task;

    pub use plan::Plan;
    pub use resource::Resource;
    pub use task::Task;
}
