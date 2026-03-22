/* Demo data for local development.

   Replace this with your own data loading (CSV, JSON, database, …). */

use std::str::FromStr;

use crate::domain::{Plan, Resource, Task};

/// Available demo datasets.
#[derive(Debug, Clone, Copy)]
pub enum DemoData {
    Small,
    Standard,
}

impl FromStr for DemoData {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "SMALL" => Ok(DemoData::Small),
            "STANDARD" => Ok(DemoData::Standard),
            _ => Err(()),
        }
    }
}

/// Generates a demo plan for the given dataset.
pub fn generate(demo: DemoData) -> Plan {
    match demo {
        DemoData::Small => generate_plan(6, 24),
        DemoData::Standard => generate_plan(12, 84),
    }
}

fn generate_plan(n_resources: usize, n_tasks: usize) -> Plan {
    let groups = ["Amber", "Blue", "Cyan", "Jade"];
    let resources: Vec<Resource> = (0..n_resources)
        .map(|i| {
            let name = format!("Resource {}", (b'A' + i as u8) as char);
            let capacity = 14 + ((i % 3) as i64 * 2);
            let affinity_group = groups[i % groups.len()];
            Resource::new(i, name, capacity, affinity_group)
        })
        .collect();

    let tasks: Vec<Task> = (0..n_tasks)
        .map(|i| {
            let demand = 1 + (i % 3) as i64;
            let preferred_group = if i < n_tasks * 36 / 100 {
                groups[0]
            } else if i < n_tasks * 59 / 100 {
                groups[1]
            } else if i < n_tasks * 80 / 100 {
                groups[2]
            } else {
                groups[3]
            };
            Task::new(
                i.to_string(),
                format!("Task {}", i + 1),
                demand,
                preferred_group,
            )
        })
        .collect();

    Plan::new(resources, tasks)
}
