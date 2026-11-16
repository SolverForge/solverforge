solverforge::planning_model! {
    root = "crates/solverforge/tests/joint_cascading_shadows/domain";

    mod crew;
    mod technician;
    mod visit;

    pub use crew::{define_constraints, Crew};
    pub use technician::Technician;
    pub use visit::Visit;
}
