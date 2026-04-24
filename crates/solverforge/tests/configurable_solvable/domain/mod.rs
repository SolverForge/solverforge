pub mod configurable {
    solverforge::planning_model! {
        root = "crates/solverforge/tests/configurable_solvable/domain/configurable";

        mod configurable_solution;
        mod dummy_entity;

        pub use configurable_solution::ConfigurableSolution;
        pub use dummy_entity::DummyEntity;
    }
}

pub mod explicit {
    solverforge::planning_model! {
        root = "crates/solverforge/tests/configurable_solvable/domain/explicit";

        mod dummy_entity;
        mod explicit_configurable_solution;

        pub use dummy_entity::DummyEntity;
        pub use explicit_configurable_solution::ExplicitConfigurableSolution;
    }
}

pub mod explicit_list {
    solverforge::planning_model! {
        root = "crates/solverforge/tests/configurable_solvable/domain/explicit_list";

        mod dummy_route;
        mod dummy_visit;
        mod explicit_list_configurable_solution;

        pub use dummy_route::DummyRoute;
        pub use dummy_visit::DummyVisit;
        pub use explicit_list_configurable_solution::ExplicitListConfigurableSolution;
    }
}

pub use configurable::ConfigurableSolution;
pub use explicit::ExplicitConfigurableSolution;
pub use explicit_list::ExplicitListConfigurableSolution;
