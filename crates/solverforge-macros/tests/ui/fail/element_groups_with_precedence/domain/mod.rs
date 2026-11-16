solverforge::planning_model! {
    root = "crates/solverforge-macros/tests/ui/fail/element_groups_with_precedence/domain";

    mod plan;
    mod route;
    mod stop;

    pub use plan::Plan;
    pub use route::Route;
    pub use stop::Stop;
}
