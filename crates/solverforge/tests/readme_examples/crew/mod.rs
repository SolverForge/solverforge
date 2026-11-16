solverforge::planning_model! {
    root = "crates/solverforge/tests/readme_examples/crew";

    mod plan;
    mod technician;
    mod visit;

    pub use plan::Crew;
    pub use technician::Technician;
    pub use visit::Visit;
}
