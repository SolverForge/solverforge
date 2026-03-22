use serde::{Deserialize, Serialize};
use solverforge::prelude::*;

/// A resource that can be assigned to tasks.
///
/// Rename this to something domain-specific (Employee, Machine, Room, …)
/// and add whatever fields your problem needs.
#[problem_fact]
#[derive(Serialize, Deserialize)]
pub struct Resource {
    /// Index into `Plan.resources` — used for O(1) joins in constraints.
    pub index: usize,
    pub name: String,
    /// Generic capacity budget consumed by assigned entities.
    pub capacity: i64,
    /// Generic affinity label for assignment preferences.
    pub affinity_group: String,
}

impl Resource {
    pub fn new(
        index: usize,
        name: impl Into<String>,
        capacity: i64,
        affinity_group: impl Into<String>,
    ) -> Self {
        Self {
            index,
            name: name.into(),
            capacity,
            affinity_group: affinity_group.into(),
        }
    }

    pub fn finalize(&mut self) {}
}
