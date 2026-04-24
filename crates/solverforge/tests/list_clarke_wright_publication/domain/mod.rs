solverforge::planning_model! {
    root = "crates/solverforge/tests/list_clarke_wright_publication/domain";

    mod customer;
    mod publication_plan;
    mod route;

    pub use customer::Customer;
    pub use publication_plan::{build_plan, PublicationPlan};
    pub use route::Route;
}
