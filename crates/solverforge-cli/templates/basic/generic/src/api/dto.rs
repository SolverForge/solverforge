use serde::{Deserialize, Serialize};

use crate::domain::{Plan, Resource, Task};
use solverforge::SolverStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDto {
    pub index: usize,
    pub name: String,
}

impl From<&Resource> for ResourceDto {
    fn from(r: &Resource) -> Self {
        Self { index: r.index, name: r.name.clone() }
    }
}

impl ResourceDto {
    pub fn to_resource(&self) -> Resource {
        Resource::new(self.index, &self.name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDto {
    pub id: String,
    pub name: String,
    pub resource: Option<ResourceDto>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDto {
    pub resources: Vec<ResourceDto>,
    pub tasks: Vec<TaskDto>,
    #[serde(default)]
    pub score: Option<String>,
    #[serde(default)]
    pub solver_status: Option<SolverStatus>,
}

impl PlanDto {
    pub fn from_plan(plan: &Plan, status: Option<SolverStatus>) -> Self {
        let resources: Vec<ResourceDto> = plan.resources.iter().map(ResourceDto::from).collect();
        let tasks: Vec<TaskDto> = plan
            .tasks
            .iter()
            .map(|t| TaskDto {
                id: t.id.clone(),
                name: t.name.clone(),
                resource: t
                    .resource_idx
                    .and_then(|idx| plan.resources.get(idx))
                    .map(ResourceDto::from),
            })
            .collect();
        Self {
            resources,
            tasks,
            score: plan.score.map(|s| s.to_string()),
            solver_status: status,
        }
    }

    pub fn to_domain(&self) -> Plan {
        let resources: Vec<Resource> =
            self.resources.iter().map(ResourceDto::to_resource).collect();
        let name_to_idx: std::collections::HashMap<&str, usize> =
            resources.iter().map(|r| (r.name.as_str(), r.index)).collect();
        let tasks: Vec<Task> = self
            .tasks
            .iter()
            .map(|t| Task {
                id: t.id.clone(),
                name: t.name.clone(),
                resource_idx: t
                    .resource
                    .as_ref()
                    .and_then(|r| name_to_idx.get(r.name.as_str()).copied()),
            })
            .collect();
        Plan::new(resources, tasks)
    }
}
