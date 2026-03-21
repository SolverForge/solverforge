use serde::{Deserialize, Serialize};
use solverforge::prelude::*;
use solverforge::CrossEntityDistanceMeter;

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
    distance_meter = "crate::domain::ItemIndexDistanceMeter",
    intra_distance_meter = "crate::domain::ItemIndexDistanceMeter",
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

/// Simple cross-entity meter for the generic list scaffold.
///
/// Uses item index distance so nearby move selectors can rank positions
/// without relying on a domain-specific metric.
#[derive(Clone, Default)]
pub struct ItemIndexDistanceMeter;

impl CrossEntityDistanceMeter<Plan> for ItemIndexDistanceMeter {
    fn distance(
        &self,
        solution: &Plan,
        src_entity: usize,
        src_pos: usize,
        dst_entity: usize,
        dst_pos: usize,
    ) -> f64 {
        let src_item = solution
            .containers
            .get(src_entity)
            .and_then(|container| container.items.get(src_pos))
            .copied();
        let dst_item = solution
            .containers
            .get(dst_entity)
            .and_then(|container| container.items.get(dst_pos))
            .copied();

        match (src_item, dst_item) {
            (Some(src), Some(dst)) => src.abs_diff(dst) as f64,
            _ => f64::INFINITY,
        }
    }
}
