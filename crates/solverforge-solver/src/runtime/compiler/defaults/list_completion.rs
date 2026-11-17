//! The list completion stage of the omitted-construction profile.

use std::fmt;

use solverforge_config::{ConstructionHeuristicConfig, ConstructionHeuristicType};
use solverforge_core::domain::PlanningSolution;

use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

use super::super::graph::{CompiledConstruction, ListConstructionKind};
use super::super::types::CompiledListSlot;
use super::stages::{
    list_config, list_policies, DefaultConstructionReason, DefaultConstructionStage,
    DefaultConstructionStepKind, DefaultPreconstructionStage, ResolvedDefaultConstructionPlan,
    ResolvedDefaultConstructionStep,
};
use super::DefaultRuntimeBindings;

/// Resolves the list completion stage: one list cheapest insertion for every
/// list slot whose `needs_completion` entry is true, in binding order.
pub(crate) fn resolve_default_list_completion<S, V, DM, IDM>(
    bindings: &DefaultRuntimeBindings<S, V, DM, IDM>,
    needs_completion: &[bool],
) -> ResolvedDefaultConstructionPlan<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    let steps = bindings
        .list_slots
        .iter()
        .zip(needs_completion)
        .filter(|(_, &needs)| needs)
        .map(|(slot, _)| list_completion_step(slot))
        .collect();
    ResolvedDefaultConstructionPlan {
        stage: DefaultConstructionStage::Preconstruction(
            DefaultPreconstructionStage::ListCompletion,
        ),
        steps,
    }
}

fn list_completion_step<S, V, DM, IDM>(
    slot: &CompiledListSlot<S, V, DM, IDM>,
) -> ResolvedDefaultConstructionStep<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    let heuristic = ConstructionHeuristicType::ListCheapestInsertion;
    let target = slot.identity();
    ResolvedDefaultConstructionStep {
        kind: DefaultConstructionStepKind::ListCompletion,
        reason: DefaultConstructionReason::ListElementsRemainUnassigned,
        construction: CompiledConstruction::List {
            kind: ListConstructionKind::CheapestInsertion,
            config: ConstructionHeuristicConfig {
                max_group_owners: slot
                    .element_group_policy()
                    .is_explicit()
                    .then_some(crate::list_groups::DEFAULT_GROUP_OWNERS),
                ..list_config(heuristic, &target)
            },
            slots: vec![slot.clone()],
        },
        required_only: false,
        target: Some(target),
        list_policies: Some(list_policies(slot)),
    }
}
