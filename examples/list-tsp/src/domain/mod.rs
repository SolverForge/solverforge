solverforge::planning_model! {
    root = "examples/list-tsp/src/domain";

    mod route;
    mod tour_plan;
    mod visit;

    pub use route::Route;
    pub use tour_plan::TourPlan;
    pub use visit::Visit;
}
