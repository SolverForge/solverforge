solverforge::planning_model! {
    root = "crates/solverforge/tests/invalid_config_lifecycle/domain";

    mod plan;
    mod resource;
    mod task;

    pub use plan::{ConfigCase, Plan};
    pub use resource::Resource;
    pub use task::Task;
}
