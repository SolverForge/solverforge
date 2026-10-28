solverforge::planning_model! {
    root = "crates/solverforge-macros/tests/ui/fail/constraint_weights_manifest_arguments/domain";

    mod task;
    mod plan;

    pub use task::Task;
    pub use plan::Plan;
}
