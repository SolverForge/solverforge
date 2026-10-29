solverforge::planning_model! {
    root = "crates/solverforge/tests/readme_examples/routes";

    mod plan;
    mod visit;

    pub use plan::{Plan, Route};
    pub use visit::Visit;
}
