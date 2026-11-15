solverforge::planning_model! {
    root = "crates/solverforge/tests/readme_examples/shop";

    mod machine;
    mod plan;
    mod task;

    pub use machine::Machine;
    pub use plan::Shop;
    pub use task::Task;
}
