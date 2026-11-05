solverforge::planning_model! {
    root = "crates/solverforge/tests/partitioned_search/domain";

    mod plan;
    mod resource;
    mod task;

    pub use plan::{take_records, Plan, Record};
    pub use resource::Resource;
    pub use task::Task;
}
