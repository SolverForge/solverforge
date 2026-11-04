solverforge::planning_model! {
    root = "crates/solverforge/tests/constraint_providers/domain";

    mod plan;
    mod resource;
    mod task;

    pub use plan::{Plan, CONSTRUCTION_TOML};
    pub use resource::Resource;
    pub use task::Task;
}
