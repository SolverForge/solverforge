solverforge::planning_model! {
    root = "crates/solverforge/tests/cascading_list_shadows/domain";

    mod machine;
    mod shop;
    mod task;

    pub use machine::Machine;
    pub use shop::{define_constraints, Shop};
    pub use task::Task;
}
