solverforge::planning_model! {
    root = "crates/solverforge/tests/local_search_config/domain";

    mod plan;
    mod task;
    mod worker;

    pub use plan::Plan;
    pub use task::Task;
    pub use worker::Worker;
}
