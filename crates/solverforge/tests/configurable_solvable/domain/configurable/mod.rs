solverforge::planning_model! {
    root = "crates/solverforge/tests/configurable_solvable/domain/configurable";

    mod configurable_solution;
    mod dummy_entity;

    pub use configurable_solution::ConfigurableSolution;
    pub use dummy_entity::DummyEntity;
}
