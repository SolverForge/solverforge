solverforge::planning_model! {
    root = "crates/solverforge-macros/tests/ui/pass/list_hooks/domain";

    mod operation;
    mod route;
    mod plan;

    pub use operation::Operation;
    pub use route::Route;
    pub use plan::Plan;
}
