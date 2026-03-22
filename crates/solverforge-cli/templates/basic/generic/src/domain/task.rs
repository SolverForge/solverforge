use serde::{Deserialize, Serialize};
use solverforge::prelude::*;

/// A task that needs to be assigned to a resource.
///
/// Rename this to something domain-specific (Shift, Job, Exam, …)
/// and add whatever fields describe a unit of work in your problem.
#[planning_entity]
#[derive(Serialize, Deserialize)]
pub struct Task {
    #[planning_id]
    pub id: String,
    pub name: String,
    /// Generic load contribution applied to the assigned resource.
    pub demand: i64,
    /// Generic affinity label for preferred assignments.
    pub preferred_group: String,
    /// Index into `Plan.resources`. `None` means unassigned.
    ///
    /// This is the planning variable the solver optimizes.
    /// Rename to match your domain (e.g. `employee_idx`, `machine_idx`).
    #[planning_variable(allows_unassigned = true)]
    pub resource_idx: Option<usize>,
}

impl Task {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        demand: i64,
        preferred_group: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            demand,
            preferred_group: preferred_group.into(),
            resource_idx: None,
        }
    }
}
