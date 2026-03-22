/* Demo data for local development.

   Replace this with your own data loading (CSV, JSON, database, …). */

use std::str::FromStr;

use crate::domain::{Container, Item, Plan};

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
        DemoData::Small => generate_plan(3, 12),
        DemoData::Standard => generate_plan(6, 36),
    }
}

fn generate_plan(n_containers: usize, n_items: usize) -> Plan {
    let containers: Vec<Container> = (0..n_containers)
        .map(|i| {
            let name = format!("Container {}", (b'A' + i as u8) as char);
            Container::new(i, name)
        })
        .collect();

    let item_facts: Vec<Item> = (0..n_items)
        .map(|i| Item::new(i, format!("Item {}", i + 1)))
        .collect();

    Plan::new(item_facts, containers)
}
