solverforge::planning_model! {
    root = "crates/solverforge/tests/configurable_solvable/domain/explicit";

    mod dummy_entity;
    mod explicit_configurable_solution;

    pub use dummy_entity::DummyEntity;
    pub use explicit_configurable_solution::ExplicitConfigurableSolution;
}
