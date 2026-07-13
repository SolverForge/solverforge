use solverforge_config::{ConstructionHeuristicConfig, ConstructionHeuristicType};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use crate::builder::{RuntimeModel, ScalarGroupBinding, VariableSlot};
use crate::phase::construction::{ScalarConstructionSchedule, ScalarOrMixedSlotOrder};

use super::graph::{CompiledConstruction, ListConstructionKind};
use super::slots::{
    matching_list_slots, matching_scalar_slots, reject_assignment_owned_scalars,
    require_list_capability, require_list_slots, require_scalar_capability, require_scalar_slots,
    resolved_scalar_bindings,
};
use super::types::{RuntimeCapability, RuntimeCompileError, RuntimeCompileErrorKind};

pub(super) fn named_scalar_group<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    group_name: &str,
    path: &str,
) -> Result<(usize, ScalarGroupBinding<S>), RuntimeCompileError> {
    model
        .scalar_groups()
        .iter()
        .enumerate()
        .find(|(_, group)| group.group_name == group_name)
        .map(|(index, group)| (index, group.clone()))
        .ok_or_else(|| RuntimeCompileError {
            path: path.to_string(),
            kind: RuntimeCompileErrorKind::MissingScalarGroup {
                group_name: group_name.to_string(),
            },
        })
}

pub(crate) fn compile_construction<S, V, DM, IDM>(
    config: &ConstructionHeuristicConfig,
    path: &str,
    descriptor: &SolutionDescriptor,
    model: &RuntimeModel<S, V, DM, IDM>,
) -> Result<CompiledConstruction<S, V, DM, IDM>, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
    V: Clone,
    DM: Clone,
    IDM: Clone,
{
    let heuristic = config.construction_heuristic_type;
    if let Some(group_name) = config.group_name.as_deref() {
        if is_list_construction(heuristic) {
            return Err(construction_shape(
                path,
                format!(
                    "grouped scalar construction group_name `{group_name}` may only be used with scalar construction heuristics"
                ),
            ));
        }
        if matches!(
            heuristic,
            ConstructionHeuristicType::AllocateEntityFromQueue
                | ConstructionHeuristicType::AllocateToValueFromQueue
        ) {
            return Err(construction_shape(
                path,
                format!(
                    "grouped scalar construction group_name `{group_name}` does not support queue-based scalar construction heuristics"
                ),
            ));
        }
        let (group_index, group) = named_scalar_group(model, group_name, path)?;
        if let Some(assignment) = group.assignment() {
            if grouped_requires_entity_order(heuristic) && !assignment.has_entity_order() {
                return Err(construction_shape(
                    path,
                    format!(
                        "assignment-backed grouped scalar construction group_name `{group_name}` with heuristic {heuristic:?} requires an entity-order capability"
                    ),
                ));
            }
            if grouped_requires_value_order(heuristic) && !assignment.has_value_order() {
                return Err(construction_shape(
                    path,
                    format!(
                        "assignment-backed grouped scalar construction group_name `{group_name}` with heuristic {heuristic:?} requires a value-order capability"
                    ),
                ));
            }
        }
        return Ok(CompiledConstruction::GroupedScalar {
            config: config.clone(),
            group_index,
            group,
            scalar_bindings: resolved_scalar_bindings(descriptor, model),
        });
    }

    if is_list_construction(heuristic) {
        let kind = ListConstructionKind::from_heuristic(heuristic)
            .expect("list-construction heuristic must map to one compiled list kind");
        let slots = require_list_slots(
            matching_list_slots(model, descriptor, &config.target, path)?,
            &config.target,
            path,
        )?;
        match kind {
            ListConstructionKind::ClarkeWright => {
                require_list_capability(&slots, RuntimeCapability::ListSavings, path)?;
            }
            ListConstructionKind::KOpt => {
                require_list_capability(&slots, RuntimeCapability::ListRoute, path)?;
            }
            ListConstructionKind::RoundRobin
            | ListConstructionKind::CheapestInsertion
            | ListConstructionKind::RegretInsertion => {}
        }
        return Ok(CompiledConstruction::List {
            kind,
            config: config.clone(),
            slots,
        });
    }

    if matches!(
        heuristic,
        ConstructionHeuristicType::FirstFit | ConstructionHeuristicType::CheapestInsertion
    ) {
        let scalar_matches = matching_scalar_slots(model, &config.target);
        reject_assignment_owned_scalars(&scalar_matches, path)?;
        let scalar_slots = scalar_matches
            .into_iter()
            .map(|(slot, _)| slot)
            .collect::<Vec<_>>();
        if !scalar_slots.is_empty() {
            require_scalar_capability(&scalar_slots, RuntimeCapability::ScalarCandidates, path)?;
        }
        let list_slots = matching_list_slots(model, descriptor, &config.target, path)?;
        if scalar_slots.is_empty()
            && list_slots.is_empty()
            && (!model.variables().is_empty()
                || config.target.entity_class.is_some()
                || config.target.variable_name.is_some())
        {
            return Err(construction_shape(
                path,
                format!(
                    "construction heuristic {heuristic:?} matched no planning variables for entity_class={:?} variable_name={:?}",
                    config.target.entity_class, config.target.variable_name
                ),
            ));
        }
        return compile_scalar_or_mixed(config, path, model, scalar_slots, list_slots);
    }

    let matched = require_scalar_slots(
        matching_scalar_slots(model, &config.target),
        &config.target,
        path,
    )?;
    reject_assignment_owned_scalars(&matched, path)?;
    let slots = matched
        .into_iter()
        .map(|(slot, _)| slot)
        .collect::<Vec<_>>();
    require_scalar_capability(&slots, RuntimeCapability::ScalarCandidates, path)?;
    if grouped_requires_entity_order(heuristic)
        || matches!(
            heuristic,
            ConstructionHeuristicType::AllocateEntityFromQueue
        )
    {
        require_scalar_capability(&slots, RuntimeCapability::ScalarEntityOrder, path)?;
    }
    if grouped_requires_value_order(heuristic)
        || matches!(
            heuristic,
            ConstructionHeuristicType::AllocateToValueFromQueue
        )
    {
        require_scalar_capability(&slots, RuntimeCapability::ScalarValueOrder, path)?;
    }
    compile_scalar_or_mixed(config, path, model, slots, Vec::new())
}

/// Freezes the scalar-construction schedule together with the
/// declaration order needed by the shared runtime-slot kernel.
///
/// Scalar-only targets use descriptor placement for both typed and dynamic
/// slots. A genuinely mixed scalar/list target uses the global slot scan.
fn compile_scalar_or_mixed<S, V, DM, IDM>(
    config: &ConstructionHeuristicConfig,
    path: &str,
    model: &RuntimeModel<S, V, DM, IDM>,
    scalar_slots: Vec<super::types::CompiledScalarSlot<S>>,
    list_slots: Vec<super::types::CompiledListSlot<S, V, DM, IDM>>,
) -> Result<CompiledConstruction<S, V, DM, IDM>, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
    V: Clone,
    DM: Clone,
    IDM: Clone,
{
    let schedule = if !scalar_slots.is_empty() && list_slots.is_empty() {
        ScalarConstructionSchedule::DescriptorPlacement
    } else {
        ScalarConstructionSchedule::GlobalRuntimeSlotScan
    };

    if matches!(schedule, ScalarConstructionSchedule::GlobalRuntimeSlotScan)
        && !matches!(
            config.construction_heuristic_type,
            ConstructionHeuristicType::FirstFit | ConstructionHeuristicType::CheapestInsertion
        )
    {
        return Err(construction_shape(
            path,
            format!(
                "construction heuristic {:?} requires descriptor-placement scalar slots; a dynamic or mixed target has no declared global-scan semantics",
                config.construction_heuristic_type
            ),
        ));
    }

    let entity_class = config.target.entity_class.as_deref();
    let variable_name = config.target.variable_name.as_deref();
    let mut scalar_index = 0usize;
    let mut list_index = 0usize;
    let mut slot_order = Vec::with_capacity(scalar_slots.len() + list_slots.len());
    for (construction_slot_index, variable) in model.variables().iter().enumerate() {
        match variable {
            VariableSlot::Scalar(slot) if slot.matches_target(entity_class, variable_name) => {
                slot_order.push(ScalarOrMixedSlotOrder::Scalar {
                    scalar_index,
                    construction_slot_index,
                });
                scalar_index += 1;
            }
            VariableSlot::DynamicScalar(slot)
                if slot.matches_target(entity_class, variable_name) =>
            {
                slot_order.push(ScalarOrMixedSlotOrder::Scalar {
                    scalar_index,
                    construction_slot_index,
                });
                scalar_index += 1;
            }
            VariableSlot::List(slot) if slot.matches_target(entity_class, variable_name) => {
                slot_order.push(ScalarOrMixedSlotOrder::List {
                    list_index,
                    construction_slot_index,
                });
                list_index += 1;
            }
            VariableSlot::DynamicList(slot) if slot.matches_target(entity_class, variable_name) => {
                slot_order.push(ScalarOrMixedSlotOrder::List {
                    list_index,
                    construction_slot_index,
                });
                list_index += 1;
            }
            VariableSlot::Scalar(_)
            | VariableSlot::DynamicScalar(_)
            | VariableSlot::List(_)
            | VariableSlot::DynamicList(_) => {}
        }
    }
    assert_eq!(
        scalar_index,
        scalar_slots.len(),
        "compiled scalar construction order must contain every matched scalar slot"
    );
    assert_eq!(
        list_index,
        list_slots.len(),
        "compiled scalar construction order must contain every matched list slot"
    );

    Ok(CompiledConstruction::ScalarOrMixed {
        config: config.clone(),
        schedule,
        scalar_slots,
        list_slots,
        slot_order,
    })
}
pub(super) fn is_list_construction(heuristic: ConstructionHeuristicType) -> bool {
    matches!(
        heuristic,
        ConstructionHeuristicType::ListRoundRobin
            | ConstructionHeuristicType::ListCheapestInsertion
            | ConstructionHeuristicType::ListRegretInsertion
            | ConstructionHeuristicType::ListClarkeWright
            | ConstructionHeuristicType::ListKOpt
    )
}

pub(super) fn grouped_requires_entity_order(heuristic: ConstructionHeuristicType) -> bool {
    matches!(
        heuristic,
        ConstructionHeuristicType::FirstFitDecreasing
            | ConstructionHeuristicType::WeakestFitDecreasing
            | ConstructionHeuristicType::StrongestFitDecreasing
    )
}

pub(super) fn grouped_requires_value_order(heuristic: ConstructionHeuristicType) -> bool {
    matches!(
        heuristic,
        ConstructionHeuristicType::WeakestFit
            | ConstructionHeuristicType::WeakestFitDecreasing
            | ConstructionHeuristicType::StrongestFit
            | ConstructionHeuristicType::StrongestFitDecreasing
    )
}

pub(super) fn construction_shape(path: &str, message: String) -> RuntimeCompileError {
    RuntimeCompileError {
        path: path.to_string(),
        kind: RuntimeCompileErrorKind::ConstructionShape { message },
    }
}
