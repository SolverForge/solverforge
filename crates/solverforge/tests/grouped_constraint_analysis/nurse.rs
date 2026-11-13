use solverforge::prelude::*;

/// A nurse shifts can be assigned to.
#[problem_fact]
pub struct Nurse {
    /// Stable nurse id.
    #[planning_id]
    pub id: usize,
}
