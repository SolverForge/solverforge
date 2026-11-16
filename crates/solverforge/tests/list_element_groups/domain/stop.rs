use solverforge::prelude::*;

/// A pick-up, a drop-off, or a single stop on a line.
#[problem_fact]
pub struct Stop {
    /// Stable stop id; job `j` is stops `2j` (pick-up) and `2j + 1`.
    #[planning_id]
    pub id: usize,
    /// The job the stop belongs to, if any.
    pub job: Option<usize>,
    /// Whether the stop is its job's pick-up.
    pub pickup: bool,
    /// Where the stop is.
    pub location: i64,
    /// The vehicle making the stop (list shadow).
    pub vehicle: Option<usize>,
    /// The stop's position on its vehicle (list shadow).
    pub index: Option<usize>,
    /// The stop before this one on its vehicle (list shadow).
    pub previous: Option<usize>,
}
