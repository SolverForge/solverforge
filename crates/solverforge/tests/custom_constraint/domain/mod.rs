solverforge::planning_model! {
    root = "crates/solverforge/tests/custom_constraint/domain";

    mod plan;
    mod resource;
    mod task;

    pub use plan::{Plan, ResourceLoad};
    pub use resource::Resource;
    pub use task::Task;
}
