solverforge::planning_model! {
    root = "crates/solverforge-macros/tests/ui/pass/constraint_weights_solution/domain";

    mod worker;
    mod task;
    mod plan;

    pub use worker::Worker;
    pub use task::Task;
    pub use plan::Plan;
}
