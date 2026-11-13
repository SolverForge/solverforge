use solverforge::prelude::*;

/// A shift on one day.
#[planning_entity]
pub struct Shift {
    /// Stable shift id.
    #[planning_id]
    pub id: usize,
    /// Day of the shift.
    pub day: usize,

    /// Assigned nurse.
    #[planning_variable(value_range_provider = "nurses")]
    pub nurse_idx: Option<usize>,
}
