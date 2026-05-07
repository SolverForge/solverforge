use solverforge::prelude::*;
use solverforge::stream::joiner::equal_bi;
use solverforge::stream::ConstraintFactory;

use super::{Machine, MachineSequence, Operation};

#[planning_solution(
    constraints = "define_constraints",
    solver_toml = "../../solver.toml"
)]
pub struct JobShopPlan {
    #[problem_fact_collection]
    pub machines: Vec<Machine>,

    #[planning_entity_collection]
    pub operations: Vec<Operation>,

    #[planning_list_element_collection(owner = "machine_sequences")]
    pub operation_values: Vec<usize>,

    #[planning_entity_collection]
    pub machine_sequences: Vec<MachineSequence>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

fn define_constraints() -> impl ConstraintSet<JobShopPlan, HardSoftScore> {
    let unassigned_machine = ConstraintFactory::<JobShopPlan, HardSoftScore>::new()
        .for_each(JobShopPlan::operations())
        .unassigned()
        .penalize_hard()
        .named("Unassigned operation machine");

    let unscheduled_operation = ConstraintFactory::<JobShopPlan, HardSoftScore>::new()
        .for_each(JobShopPlan::operations())
        .if_not_exists((
            ConstraintFactory::<JobShopPlan, HardSoftScore>::new()
                .for_each(JobShopPlan::machine_sequences())
                .flattened(|machine: &MachineSequence| &machine.operations),
            equal_bi(
                |operation: &Operation| operation.id,
                |assigned: &usize| *assigned,
            ),
        ))
        .penalize_hard()
        .named("Unscheduled operation");

    let same_job_same_machine = ConstraintFactory::<JobShopPlan, HardSoftScore>::new()
        .for_each(JobShopPlan::operations())
        .join((
            ConstraintFactory::<JobShopPlan, HardSoftScore>::new()
                .for_each(JobShopPlan::operations()),
            |left: &Operation, right: &Operation| {
                left.id < right.id
                    && left.job == right.job
                    && left.machine_idx.is_some()
                    && left.machine_idx == right.machine_idx
            },
        ))
        .penalize_soft()
        .named("Same job machine reuse");

    (
        unassigned_machine,
        unscheduled_operation,
        same_job_same_machine,
    )
}
