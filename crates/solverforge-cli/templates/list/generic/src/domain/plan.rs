use serde::{Deserialize, Serialize};
use solverforge::prelude::*;

use super::{Container, Item};

/// The root planning solution: items + containers + score.
///
/// Rename this to something domain-specific (Route, Schedule, Assignment, …).
#[planning_solution(constraints = "crate::constraints::create_constraints")]
#[shadow_variable_updates(
    list_owner = "containers",
    list_field = "items",
    element_type = "usize",
    element_collection = "all_item_indices",
)]
#[derive(Serialize, Deserialize)]
pub struct Plan {
    #[problem_fact_collection]
    pub item_facts: Vec<Item>,
    #[planning_entity_collection]
    pub containers: Vec<Container>,
    pub all_item_indices: Vec<usize>,
    #[planning_score]
    pub score: Option<HardSoftScore>,
}

impl Plan {
    pub fn new(item_facts: Vec<Item>, containers: Vec<Container>) -> Self {
        let all_item_indices = (0..item_facts.len()).collect();
        Self { item_facts, containers, all_item_indices, score: None }
    }
}
