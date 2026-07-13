use solverforge_config::{MoveSelectorConfig, SelectionOrder, VariableTargetConfig};
use solverforge_core::domain::PlanningSolution;

use crate::builder::{
    RuntimeCandidateMetricBinding, RuntimeModel, RuntimeProviderHandle, RuntimeScalarSlot,
    RuntimeScalarSlotId, ScalarGroupBinding, VariableSlot,
};

use super::graph::{
    CompiledProviderPlan, CompiledSelectorNode, ProviderBindingPlan, ProviderBindingPolicy,
    ProviderMoveKind, ProviderPullTiming, ProviderSchedule, PYTHON_PROVIDER_CANDIDATE_CONTRACT,
    REPAIR_PROVIDER_ROTATION_SALT, STATIC_REPAIR_CONSTRAINT_ROTATION_SALT,
    STATIC_REPAIR_PROVIDER_ROTATION_SALT, STATIC_REPAIR_SPEC_ROTATION_SALT,
};
use super::types::{RuntimeCompileError, RuntimeCompileErrorKind};

pub(super) fn compile_conflict_repair<S, V, DM, IDM>(
    config: &MoveSelectorConfig,
    candidate_order: SelectionOrder,
    candidate_metric: Option<RuntimeCandidateMetricBinding<S>>,
    path: &str,
    model: &RuntimeModel<S, V, DM, IDM>,
) -> Result<CompiledSelectorNode<S, V, DM, IDM>, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
{
    let (
        constraints,
        max_matches_per_step,
        max_repairs_per_match,
        max_moves_per_step,
        include_soft_matches,
        move_kind,
    ) = match config {
        MoveSelectorConfig::ConflictRepairMoveSelector(repair) => (
            &repair.constraints,
            repair.max_matches_per_step,
            repair.max_repairs_per_match,
            repair.max_moves_per_step,
            repair.include_soft_matches,
            ProviderMoveKind::ConflictRepair,
        ),
        MoveSelectorConfig::CompoundConflictRepairMoveSelector(repair) => (
            &repair.constraints,
            repair.max_matches_per_step,
            repair.max_repairs_per_match,
            repair.max_moves_per_step,
            repair.include_soft_matches,
            ProviderMoveKind::CompoundConflictRepair,
        ),
        _ => unreachable!("compile_conflict_repair requires a conflict-repair selector"),
    };

    let registry = model.runtime_provider_registry();
    let callback_has_match = registry.repairs().iter().any(|provider| {
        provider.declared_constraints.iter().any(|declared| {
            constraints
                .iter()
                .any(|configured| configured == declared.as_ref())
        })
    });
    let static_has_match = registry.static_repairs().iter().any(|provider| {
        constraints
            .iter()
            .any(|configured| configured == provider.repair.constraint_name())
    });
    if !callback_has_match && !static_has_match {
        return Err(RuntimeCompileError {
            path: path.to_string(),
            kind: RuntimeCompileErrorKind::MissingConflictRepairProvider,
        });
    }
    let callback_allowed = if !registry.repairs().is_empty() {
        // Structural assignment exclusion happens even when the configured
        // constraints later select no callback provider. No decorator/pull can
        // silently narrow a generic dynamic callback universe.
        Some(unscoped_dynamic_provider_slots(model, path)?)
    } else {
        None
    };
    let static_allowed = if !registry.static_repairs().is_empty() {
        Some(static_non_assignment_provider_slots(model, path)?)
    } else {
        None
    };
    let mut bindings = Vec::new();
    if let Some(allowed_slots) = callback_allowed {
        bindings.extend(
            registry
                .repairs()
                .iter()
                .enumerate()
                .map(|(index, provider)| ProviderBindingPlan {
                    handle: RuntimeProviderHandle::CallbackRepair(index),
                    declared_schema_index: provider.declared_index,
                    allowed_slots: allowed_slots.clone(),
                    policy: ProviderBindingPolicy::CallbackRepair {
                        rotation_seed_salt: REPAIR_PROVIDER_ROTATION_SALT
                            ^ max_moves_per_step as u64,
                        pull_timing: ProviderPullTiming::FirstReachableNext,
                    },
                    candidate_contract: PYTHON_PROVIDER_CANDIDATE_CONTRACT,
                }),
        );
    }
    if let Some(allowed_slots) = static_allowed {
        bindings.extend(
            registry
                .static_repairs()
                .iter()
                .enumerate()
                .map(|(index, provider)| ProviderBindingPlan {
                    handle: RuntimeProviderHandle::StaticRepair(index),
                    declared_schema_index: provider.declared_index,
                    allowed_slots: allowed_slots.clone(),
                    policy: ProviderBindingPolicy::StaticRepair {
                        constraint_rotation_seed_salt: STATIC_REPAIR_CONSTRAINT_ROTATION_SALT
                            ^ max_moves_per_step as u64,
                        provider_rotation_seed_salt: STATIC_REPAIR_PROVIDER_ROTATION_SALT,
                        spec_rotation_seed_salt: STATIC_REPAIR_SPEC_ROTATION_SALT,
                        pull_timing: ProviderPullTiming::OpenCursor,
                    },
                    candidate_contract: PYTHON_PROVIDER_CANDIDATE_CONTRACT,
                }),
        );
    }
    Ok(CompiledSelectorNode::Provider {
        config: config.clone(),
        candidate_order,
        candidate_metric,
        plan: CompiledProviderPlan {
            move_kind,
            schedule: ProviderSchedule::Repair {
                constraints: constraints.clone(),
                max_matches_per_step,
                max_repairs_per_match,
                max_moves_per_step,
                include_soft_matches,
            },
            bindings,
        },
    })
}

/// Binds the complete generic-Python dynamic scalar universe in declared
/// model order.  Assignment ownership is a compile-time error—not a runtime
/// filter—so a limited/unreached provider can never observe a changed set of
/// legal callback targets.
pub(super) fn unscoped_dynamic_provider_slots<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    path: &str,
) -> Result<Vec<RuntimeScalarSlotId>, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
{
    let mut slots = Vec::new();
    for variable in model.variables() {
        let VariableSlot::DynamicScalar(slot) = variable else {
            continue;
        };
        if model.assignment_group_covers_dynamic_scalar_variable(slot) {
            return Err(RuntimeCompileError {
                path: path.to_string(),
                kind: RuntimeCompileErrorKind::AssignmentOwnedScalar {
                    slot: RuntimeScalarSlot::Dynamic(slot.clone()).id(),
                },
            });
        }
        slots.push(RuntimeScalarSlotId::from_dynamic_slot(slot));
    }
    if slots.is_empty() {
        return Err(RuntimeCompileError {
            path: path.to_string(),
            kind: RuntimeCompileErrorKind::NoMatchingScalarSlot {
                target: VariableTargetConfig::default(),
            },
        });
    }
    Ok(slots)
}

/// Native repair adapters retain the existing typed non-assignment universe.
/// This is a source policy inside the common provider cursor, not a second
/// selector implementation; its slots still use the same `RuntimeScalarSlot`
/// identity/resolver as callback providers.
pub(super) fn static_non_assignment_provider_slots<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    path: &str,
) -> Result<Vec<RuntimeScalarSlotId>, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
{
    let slots = model
        .variables()
        .iter()
        .filter_map(|variable| match variable {
            VariableSlot::Scalar(slot) if !model.assignment_group_covers_scalar_variable(slot) => {
                Some(RuntimeScalarSlot::Static(*slot).id())
            }
            VariableSlot::Scalar(_)
            | VariableSlot::DynamicScalar(_)
            | VariableSlot::List(_)
            | VariableSlot::DynamicList(_) => None,
        })
        .collect::<Vec<_>>();
    if slots.is_empty() {
        return Err(RuntimeCompileError {
            path: path.to_string(),
            kind: RuntimeCompileErrorKind::NoMatchingScalarSlot {
                target: VariableTargetConfig::default(),
            },
        });
    }
    Ok(slots)
}

/// Resolves a named typed group to the same canonical IDs used by provider
/// edits.  No candidate callback may bypass this membership boundary.
pub(super) fn scalar_group_provider_slots<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    group: &ScalarGroupBinding<S>,
    path: &str,
) -> Result<Vec<RuntimeScalarSlotId>, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
{
    let mut slots = Vec::with_capacity(group.members.len());
    for member in &group.members {
        let slot = model
            .variables()
            .iter()
            .find_map(|variable| match variable {
                VariableSlot::Scalar(slot)
                    if slot.descriptor_index == member.descriptor_index
                        && slot.variable_index == member.variable_index =>
                {
                    Some(RuntimeScalarSlot::Static(*slot).id())
                }
                VariableSlot::DynamicScalar(slot)
                    if slot.descriptor_index() == member.descriptor_index
                        && slot.descriptor_variable_index() == member.variable_index =>
                {
                    Some(RuntimeScalarSlot::Dynamic(slot.clone()).id())
                }
                VariableSlot::Scalar(_)
                | VariableSlot::DynamicScalar(_)
                | VariableSlot::List(_)
                | VariableSlot::DynamicList(_) => None,
            })
            .ok_or_else(|| RuntimeCompileError {
                path: path.to_string(),
                kind: RuntimeCompileErrorKind::InvalidSlotIdentity {
                    message: format!(
                        "scalar group `{}` member {}.{} has no canonical runtime scalar slot",
                        group.group_name, member.entity_type_name, member.variable_name
                    ),
                },
            })?;
        slots.push(slot);
    }
    Ok(slots)
}
