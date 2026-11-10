solverforge::planning_model! {
    root = "crates/solverforge/tests/configurable_solvable/domain/explicit_list";

    mod dummy_route;
    mod dummy_visit;
    mod explicit_list_configurable_solution;

    pub use dummy_route::DummyRoute;
    pub use dummy_visit::DummyVisit;
    pub use explicit_list_configurable_solution::ExplicitListConfigurableSolution;
}
