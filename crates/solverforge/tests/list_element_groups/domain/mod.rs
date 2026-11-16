solverforge::planning_model! {
    root = "crates/solverforge/tests/list_element_groups/domain";

    mod deliveries;
    mod stop;
    mod vehicle;

    pub use deliveries::Deliveries;
    pub use stop::Stop;
    pub use vehicle::Vehicle;
}
