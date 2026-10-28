solverforge::planning_model! {
    root = "crates/solverforge/tests/constraint_weights";

    mod plan;
    mod task;
    mod worker;

    pub use plan::{Plan, COST, PREFERRED};
    pub use task::Task;
    pub use worker::Worker;
}
