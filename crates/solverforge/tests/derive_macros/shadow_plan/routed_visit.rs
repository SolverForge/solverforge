use solverforge::prelude::*;

#[problem_fact]
pub struct RoutedVisit {
    #[planning_id]
    pub id: usize,
    pub route: Option<usize>,
}
