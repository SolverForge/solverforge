use solverforge_config::{MoveSelectorConfig, SelectionOrder, VariableTargetConfig};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor, VariableType};

use crate::builder::context::list_access::ListAccessCapability;
use crate::builder::{
    ListVariableSlot, RuntimeCandidateMetricBinding, RuntimeModel, ScalarAccessCapability,
    VariableSlot,
};
use crate::descriptor::{collect_bindings, ResolvedVariableBinding};

use super::graph::{CompiledSelectorNode, ListLeafKind, ScalarLeafKind};
use super::types::{
    CompiledListSlot, CompiledScalarSlot, RuntimeCapability, RuntimeCompileError,
    RuntimeCompileErrorKind,
};

pub(crate) fn matching_scalar_slots<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    target: &VariableTargetConfig,
) -> Vec<(CompiledScalarSlot<S>, bool)>
where
    S: PlanningSolution + 'static,
{
    // Preserve the authoritative `RuntimeModel.variables()` order.  Typed
    // then dynamic concatenation would silently re-order a hybrid model's
    // targetless selector and change candidate order/seeded behavior.
    model
        .variables()
        .iter()
        .filter_map(|variable| match variable {
            VariableSlot::Scalar(slot)
                if slot.matches_target(
                    target.entity_class.as_deref(),
                    target.variable_name.as_deref(),
                ) =>
            {
                let assignment_owned = model.assignment_group_covers_scalar_variable(slot);
                Some((CompiledScalarSlot::Static(*slot), assignment_owned))
            }
            VariableSlot::DynamicScalar(slot)
                if slot.matches_target(
                    target.entity_class.as_deref(),
                    target.variable_name.as_deref(),
                ) =>
            {
                let assignment_owned = model.assignment_group_covers_dynamic_scalar_variable(slot);
                Some((CompiledScalarSlot::Dynamic(slot.clone()), assignment_owned))
            }
            VariableSlot::List(_)
            | VariableSlot::DynamicList(_)
            | VariableSlot::Scalar(_)
            | VariableSlot::DynamicScalar(_) => None,
        })
        .collect()
}

/// Freezes the descriptor bindings used by grouped scalar construction.
///
pub(crate) fn resolved_scalar_bindings<S, V, DM, IDM>(
    descriptor: &SolutionDescriptor,
    _model: &RuntimeModel<S, V, DM, IDM>,
) -> Vec<ResolvedVariableBinding<S>>
where
    S: PlanningSolution + 'static,
{
    collect_bindings(descriptor)
        .into_iter()
        .map(ResolvedVariableBinding::new)
        .collect()
}

pub(crate) fn matching_list_slots<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    descriptor: &SolutionDescriptor,
    target: &VariableTargetConfig,
    path: &str,
) -> Result<Vec<CompiledListSlot<S, V, DM, IDM>>, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
    V: Clone,
    DM: Clone,
    IDM: Clone,
{
    // See `matching_scalar_slots`: a mixed typed/dynamic list model also has
    // one declaration order, and the compiler must retain it exactly.
    let mut matched = Vec::new();
    for variable in model.variables() {
        match variable {
            VariableSlot::List(slot)
                if slot.matches_target(
                    target.entity_class.as_deref(),
                    target.variable_name.as_deref(),
                ) =>
            {
                let variable_index = typed_list_variable_index(descriptor, slot, path)?;
                matched.push(CompiledListSlot::from_static(slot.clone(), variable_index));
            }
            VariableSlot::DynamicList(slot)
                if slot.matches_target(
                    target.entity_class.as_deref(),
                    target.variable_name.as_deref(),
                ) =>
            {
                matched.push(CompiledListSlot::from_dynamic(slot.clone()))
            }
            VariableSlot::Scalar(_)
            | VariableSlot::DynamicScalar(_)
            | VariableSlot::List(_)
            | VariableSlot::DynamicList(_) => {}
        }
    }
    Ok(matched)
}

/// Resolves the real descriptor-local coordinate for a native list slot.
///
/// Native list slots historically stored only their owning descriptor index.
/// Treating the list variable as index zero is unsound whenever the descriptor
/// has another genuine field before it, and it would poison candidate identity
/// and trace provenance.  The descriptor is the one canonical source of this
/// coordinate, so a malformed binding is rejected rather than guessed.
pub(super) fn typed_list_variable_index<S, V, DM, IDM>(
    descriptor: &SolutionDescriptor,
    slot: &ListVariableSlot<S, V, DM, IDM>,
    path: &str,
) -> Result<usize, RuntimeCompileError> {
    let Some(entity) = descriptor.entity_descriptors.get(slot.descriptor_index) else {
        return Err(RuntimeCompileError {
            path: path.to_string(),
            kind: RuntimeCompileErrorKind::InvalidSlotIdentity {
                message: format!(
                    "native list slot {}.{} refers to missing descriptor {}",
                    slot.entity_type_name, slot.variable_name, slot.descriptor_index
                ),
            },
        });
    };
    if entity.type_name != slot.entity_type_name {
        return Err(RuntimeCompileError {
            path: path.to_string(),
            kind: RuntimeCompileErrorKind::InvalidSlotIdentity {
                message: format!(
                    "native list slot {}.{} is bound to descriptor {} ({})",
                    slot.entity_type_name,
                    slot.variable_name,
                    slot.descriptor_index,
                    entity.type_name
                ),
            },
        });
    }
    let Some((variable_index, variable)) = entity
        .variable_descriptors
        .iter()
        .enumerate()
        .find(|(_, variable)| variable.name == slot.variable_name)
    else {
        return Err(RuntimeCompileError {
            path: path.to_string(),
            kind: RuntimeCompileErrorKind::InvalidSlotIdentity {
                message: format!(
                    "native list slot {}.{} has no descriptor variable",
                    slot.entity_type_name, slot.variable_name
                ),
            },
        });
    };
    if variable.variable_type != VariableType::List {
        return Err(RuntimeCompileError {
            path: path.to_string(),
            kind: RuntimeCompileErrorKind::InvalidSlotIdentity {
                message: format!(
                    "native list slot {}.{} resolves to a non-list descriptor variable",
                    slot.entity_type_name, slot.variable_name
                ),
            },
        });
    }
    Ok(variable_index)
}

pub(super) fn require_scalar_slots<S>(
    slots: Vec<(CompiledScalarSlot<S>, bool)>,
    target: &VariableTargetConfig,
    path: &str,
) -> Result<Vec<(CompiledScalarSlot<S>, bool)>, RuntimeCompileError> {
    if slots.is_empty() {
        return Err(RuntimeCompileError {
            path: path.to_string(),
            kind: RuntimeCompileErrorKind::NoMatchingScalarSlot {
                target: target.clone(),
            },
        });
    }
    Ok(slots)
}

pub(super) fn require_list_slots<S, V, DM, IDM>(
    slots: Vec<CompiledListSlot<S, V, DM, IDM>>,
    target: &VariableTargetConfig,
    path: &str,
) -> Result<Vec<CompiledListSlot<S, V, DM, IDM>>, RuntimeCompileError> {
    if slots.is_empty() {
        return Err(RuntimeCompileError {
            path: path.to_string(),
            kind: RuntimeCompileErrorKind::NoMatchingListSlot {
                target: target.clone(),
            },
        });
    }
    Ok(slots)
}

pub(super) fn reject_assignment_owned_scalars<S>(
    slots: &[(CompiledScalarSlot<S>, bool)],
    path: &str,
) -> Result<(), RuntimeCompileError> {
    if let Some((slot, _)) = slots.iter().find(|(_, assignment_owned)| *assignment_owned) {
        return Err(RuntimeCompileError {
            path: path.to_string(),
            kind: RuntimeCompileErrorKind::AssignmentOwnedScalar { slot: slot.id() },
        });
    }
    Ok(())
}

pub(super) fn require_scalar_capability<S>(
    slots: &[CompiledScalarSlot<S>],
    capability: RuntimeCapability,
    path: &str,
) -> Result<(), RuntimeCompileError> {
    let scalar_capability = match capability {
        RuntimeCapability::ScalarCandidates => ScalarAccessCapability::Candidates,
        RuntimeCapability::ScalarNearbyValue => ScalarAccessCapability::NearbyValue,
        RuntimeCapability::ScalarNearbyEntity => ScalarAccessCapability::NearbyEntity,
        RuntimeCapability::ScalarEntityOrder => ScalarAccessCapability::ConstructionEntityOrder,
        RuntimeCapability::ScalarValueOrder => ScalarAccessCapability::ConstructionValueOrder,
        _ => unreachable!("scalar capability validation received a list capability"),
    };
    if let Some(slot) = slots
        .iter()
        .find(|slot| !slot.has_capability(scalar_capability))
    {
        return Err(RuntimeCompileError {
            path: path.to_string(),
            kind: RuntimeCompileErrorKind::MissingCapability {
                slot: slot.id(),
                capability,
            },
        });
    }
    Ok(())
}

pub(super) fn require_list_capability<S, V, DM, IDM>(
    slots: &[CompiledListSlot<S, V, DM, IDM>],
    capability: RuntimeCapability,
    path: &str,
) -> Result<(), RuntimeCompileError> {
    let access_capability = match capability {
        RuntimeCapability::ListSet => ListAccessCapability::Set,
        RuntimeCapability::ListReverse => ListAccessCapability::Reverse,
        RuntimeCapability::ListSublist => ListAccessCapability::Sublist,
        RuntimeCapability::ListPrecedence => ListAccessCapability::Precedence,
        RuntimeCapability::ListCrossPositionDistance => ListAccessCapability::CrossPositionDistance,
        RuntimeCapability::ListIntraPositionDistance => ListAccessCapability::IntraPositionDistance,
        RuntimeCapability::ListRoute => ListAccessCapability::Route,
        RuntimeCapability::ListSavings => ListAccessCapability::Savings,
        _ => unreachable!("list capability validation received a scalar capability"),
    };
    if let Some(slot) = slots.iter().find(|slot| !slot.supports(access_capability)) {
        return Err(RuntimeCompileError {
            path: path.to_string(),
            kind: RuntimeCompileErrorKind::MissingCapability {
                slot: slot.identity(),
                capability,
            },
        });
    }
    Ok(())
}

pub(super) fn compile_scalar_leaf<S, V, DM, IDM>(
    kind: ScalarLeafKind,
    config: &MoveSelectorConfig,
    candidate_order: SelectionOrder,
    candidate_metric: Option<RuntimeCandidateMetricBinding<S>>,
    target: &VariableTargetConfig,
    path: &str,
    model: &RuntimeModel<S, V, DM, IDM>,
) -> Result<CompiledSelectorNode<S, V, DM, IDM>, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
{
    let matched = require_scalar_slots(matching_scalar_slots(model, target), target, path)?;
    reject_assignment_owned_scalars(&matched, path)?;
    let slots = matched
        .into_iter()
        .map(|(slot, _)| slot)
        .collect::<Vec<_>>();

    let required = match kind {
        ScalarLeafKind::Change
        | ScalarLeafKind::PillarChange
        | ScalarLeafKind::PillarSwap
        | ScalarLeafKind::RuinRecreate => Some(RuntimeCapability::ScalarCandidates),
        ScalarLeafKind::NearbyChange => Some(RuntimeCapability::ScalarNearbyValue),
        ScalarLeafKind::NearbySwap => Some(RuntimeCapability::ScalarNearbyEntity),
        ScalarLeafKind::Swap => None,
    };
    if let Some(capability) = required {
        require_scalar_capability(&slots, capability, path)?;
    }

    Ok(CompiledSelectorNode::Scalar {
        kind,
        config: config.clone(),
        candidate_order,
        candidate_metric,
        slots,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn compile_list_leaf<S, V, DM, IDM>(
    kind: ListLeafKind,
    config: &MoveSelectorConfig,
    candidate_order: SelectionOrder,
    candidate_metric: Option<RuntimeCandidateMetricBinding<S>>,
    target: &VariableTargetConfig,
    path: &str,
    descriptor: &SolutionDescriptor,
    model: &RuntimeModel<S, V, DM, IDM>,
) -> Result<CompiledSelectorNode<S, V, DM, IDM>, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
    V: Clone,
    DM: Clone,
    IDM: Clone,
{
    let slots = require_list_slots(
        matching_list_slots(model, descriptor, target, path)?,
        target,
        path,
    )?;
    let mut required = Vec::new();
    match kind {
        ListLeafKind::Change | ListLeafKind::Ruin => {}
        ListLeafKind::NearbyChange => {
            required.push(RuntimeCapability::ListCrossPositionDistance);
        }
        ListLeafKind::Swap => required.push(RuntimeCapability::ListSet),
        ListLeafKind::Permute | ListLeafKind::SublistChange | ListLeafKind::SublistSwap => {
            required.push(RuntimeCapability::ListSublist)
        }
        ListLeafKind::Precedence => {
            required.extend([
                RuntimeCapability::ListPrecedence,
                RuntimeCapability::ListSet,
                RuntimeCapability::ListReverse,
                RuntimeCapability::ListSublist,
            ]);
        }
        ListLeafKind::NearbySwap => {
            required.extend([
                RuntimeCapability::ListCrossPositionDistance,
                RuntimeCapability::ListSet,
            ]);
        }
        ListLeafKind::Reverse => required.push(RuntimeCapability::ListReverse),
        ListLeafKind::KOpt => {
            required.push(RuntimeCapability::ListSublist);
            if matches!(config, MoveSelectorConfig::KOptMoveSelector(kopt) if kopt.max_nearby > 0) {
                required.push(RuntimeCapability::ListIntraPositionDistance);
            }
        }
    }
    for capability in required {
        require_list_capability(&slots, capability, path)?;
    }

    Ok(CompiledSelectorNode::List {
        kind,
        config: config.clone(),
        candidate_order,
        candidate_metric,
        slots,
    })
}
