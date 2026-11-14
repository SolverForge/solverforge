use solverforge::prelude::*;

/// A nurse, identified by name: a `Display` planning id.
#[problem_fact]
pub struct Nurse {
    /// Stable nurse name.
    #[planning_id]
    pub name: String,
    /// Index of the nurse in the roster, which shifts refer to.
    pub index: usize,
    /// The day the nurse cannot work.
    pub unavailable_day: usize,
}
