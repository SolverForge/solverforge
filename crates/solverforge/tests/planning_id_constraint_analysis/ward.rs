use solverforge::prelude::*;

/// A ward with no planning id.
#[problem_fact]
pub struct Ward {
    /// Ward name, not a planning id.
    pub name: String,
    /// Whether the ward lacks a head nurse.
    pub unstaffed: bool,
}
