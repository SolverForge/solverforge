solverforge::planning_model! {
    root = "crates/solverforge/tests/facade_reachability/domain";

    mod plan;
    mod resource;
    mod task;

    pub use plan::{Plan, PROBE_PHASE_DRAWS};
    pub use resource::Resource;
    pub use task::Task;
}
