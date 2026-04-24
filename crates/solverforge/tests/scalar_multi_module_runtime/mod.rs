solverforge::planning_model! {
    root = "crates/solverforge/tests/scalar_multi_module_runtime";

    mod plan;
    mod task;
    mod worker;

    pub use plan::Plan;
    pub use task::{Task, WorkTask};
    pub use worker::Worker;
}
