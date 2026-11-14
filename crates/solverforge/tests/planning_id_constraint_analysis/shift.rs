use solverforge::prelude::*;

/// A shift id with no `Display` implementation: its planning id renders with
/// `Debug`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShiftId(pub u32);

/// A shift on one day.
#[planning_entity]
pub struct Shift {
    /// Stable shift id.
    #[planning_id]
    pub id: ShiftId,
    /// Day of the shift.
    pub day: usize,

    /// Assigned nurse.
    #[planning_variable(value_range_provider = "nurses")]
    pub nurse_idx: Option<usize>,
}
