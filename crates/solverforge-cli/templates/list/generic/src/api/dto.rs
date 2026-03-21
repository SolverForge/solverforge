use serde::{Deserialize, Serialize};

use crate::domain::{Container, Item, Plan};
use solverforge::SolverStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDto {
    pub index: usize,
    pub name: String,
}

impl From<&Item> for ItemDto {
    fn from(i: &Item) -> Self {
        Self { index: i.index, name: i.name.clone() }
    }
}

impl ItemDto {
    pub fn to_item(&self) -> Item {
        Item::new(self.index, &self.name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerDto {
    pub id: usize,
    pub name: String,
    /// Item names in sequence order.
    pub items: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDto {
    pub items: Vec<ItemDto>,
    pub containers: Vec<ContainerDto>,
    #[serde(default)]
    pub score: Option<String>,
    #[serde(default)]
    pub solver_status: Option<SolverStatus>,
}

/// Constraint analysis result.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintAnalysisDto {
    pub name: String,
    #[serde(rename = "type")]
    pub constraint_type: String,
    pub weight: String,
    pub score: String,
    pub matches: Vec<ConstraintMatchDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintMatchDto {
    pub score: String,
    pub justification: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeResponse {
    pub score: String,
    pub constraints: Vec<ConstraintAnalysisDto>,
}

impl PlanDto {
    pub fn from_plan(plan: &Plan, status: Option<SolverStatus>) -> Self {
        let items: Vec<ItemDto> = plan.item_facts.iter().map(ItemDto::from).collect();
        let containers: Vec<ContainerDto> = plan
            .containers
            .iter()
            .map(|c| ContainerDto {
                id: c.id,
                name: c.name.clone(),
                items: c
                    .items
                    .iter()
                    .filter_map(|&idx| plan.item_facts.get(idx))
                    .map(|item| item.name.clone())
                    .collect(),
            })
            .collect();
        Self {
            items,
            containers,
            score: plan.score.map(|s| s.to_string()),
            solver_status: status,
        }
    }

    pub fn to_domain(&self) -> Plan {
        let item_facts: Vec<Item> = self.items.iter().map(ItemDto::to_item).collect();
        let name_to_idx: std::collections::HashMap<&str, usize> =
            item_facts.iter().map(|i| (i.name.as_str(), i.index)).collect();
        let containers: Vec<Container> = self
            .containers
            .iter()
            .map(|c| {
                let items: Vec<usize> = c
                    .items
                    .iter()
                    .filter_map(|name| name_to_idx.get(name.as_str()).copied())
                    .collect();
                Container { id: c.id, name: c.name.clone(), items }
            })
            .collect();
        Plan::new(item_facts, containers)
    }
}
