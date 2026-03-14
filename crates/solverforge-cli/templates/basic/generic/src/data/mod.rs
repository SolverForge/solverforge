/* Demo data for local development.

   Replace this with your own data loading (CSV, JSON, database, …). */

use crate::domain::{Plan, Resource, Task};

/// Returns a small demo plan so the app runs out of the box.
pub fn demo_plan() -> Plan {
    let resources = vec![
        Resource::new(0, "Resource A"),
        Resource::new(1, "Resource B"),
        Resource::new(2, "Resource C"),
    ];
    let tasks = (0..9)
        .map(|i| Task::new(i.to_string(), format!("Task {}", i + 1)))
        .collect();
    Plan::new(resources, tasks)
}
