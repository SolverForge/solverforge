solverforge::planning_model! {
    root = "crates/solverforge/tests/derive_macros/duplicate_names";

    mod plan;
    mod plain_route;
    mod route;
    mod visit;

    pub use plan::Plan;
    pub use plain_route::RenamedPlainRoute;
    pub use route::Route;
    pub use visit::Visit;
}
