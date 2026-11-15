solverforge::planning_model! {
    root = "crates/solverforge-macros/tests/ui/fail/cascading_shadow_without_list_structure/domain";

    mod machine;
    mod shop;
    mod task;

    pub use machine::Machine;
    pub use shop::Shop;
    pub use task::Task;
}
