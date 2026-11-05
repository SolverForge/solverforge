solverforge::planning_model! {
    root = "crates/solverforge/tests/construction_heuristics/domain";

    mod plan;
    mod task;
    mod worker;

    pub use plan::Plan;
    pub use task::Task;
    pub use worker::Worker;
}
