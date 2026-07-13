use solverforge_config::{
    ChangeMoveConfig, CompoundConflictRepairMoveSelectorConfig, GroupedScalarMoveSelectorConfig,
    MoveSelectorConfig, NearbyChangeMoveConfig, NearbySwapMoveConfig, SelectionOrder,
    SwapMoveConfig,
};
use solverforge_core::domain::PlanningSolution;

use crate::builder::{RuntimeModel, ScalarAccessCapability};

use super::super::super::defaults::DefaultScalarBinding;
use super::super::super::types::RuntimeCompileError;
use super::super::{
    target_for, DefaultLocalSearchSelectorDeclaration, DefaultLocalSearchSelectorFamily,
    DefaultSelectorCapabilityPolicy,
};
use super::default_selector_error;

const DEFAULT_SCALAR_NEARBY_LIMIT: usize = 10;

pub(super) fn append_nearby_scalar_policy<S>(
    scalar_slots: &[DefaultScalarBinding<S>],
    declarations: &mut Vec<DefaultLocalSearchSelectorDeclaration>,
) {
    for dynamic in [false, true] {
        for binding in scalar_slots.iter().filter(|binding| {
            !binding.assignment_owned
                && binding.slot.is_dynamic() == dynamic
                && binding
                    .slot
                    .has_capability(ScalarAccessCapability::NearbyValue)
        }) {
            push_scalar(
                declarations,
                DefaultLocalSearchSelectorFamily::NearbyScalarChange,
                MoveSelectorConfig::NearbyChangeMoveSelector(NearbyChangeMoveConfig {
                    selection_order: Some(SelectionOrder::Random),
                    selection_metric: None,
                    max_nearby: DEFAULT_SCALAR_NEARBY_LIMIT,
                    value_candidate_limit: None,
                    target: target_for(&binding.slot.id()),
                }),
                &binding.slot,
            );
        }
        for binding in scalar_slots.iter().filter(|binding| {
            !binding.assignment_owned
                && binding.slot.is_dynamic() == dynamic
                && binding
                    .slot
                    .has_capability(ScalarAccessCapability::NearbyEntity)
        }) {
            push_scalar(
                declarations,
                DefaultLocalSearchSelectorFamily::NearbyScalarSwap,
                MoveSelectorConfig::NearbySwapMoveSelector(NearbySwapMoveConfig {
                    selection_order: Some(SelectionOrder::Random),
                    selection_metric: None,
                    max_nearby: DEFAULT_SCALAR_NEARBY_LIMIT,
                    target: target_for(&binding.slot.id()),
                }),
                &binding.slot,
            );
        }
    }
}

pub(super) fn append_ordinary_scalar_policy<S>(
    scalar_slots: &[DefaultScalarBinding<S>],
    declarations: &mut Vec<DefaultLocalSearchSelectorDeclaration>,
) {
    // The established typed order is retained. Dynamic scalar slots receive
    // the same ordinary change/swap pair instead of silently losing swap.
    for dynamic in [false, true] {
        for binding in scalar_slots
            .iter()
            .filter(|binding| !binding.assignment_owned && binding.slot.is_dynamic() == dynamic)
        {
            push_scalar(
                declarations,
                DefaultLocalSearchSelectorFamily::ScalarChange,
                MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                    selection_order: Some(SelectionOrder::Random),
                    selection_metric: None,
                    value_candidate_limit: None,
                    target: target_for(&binding.slot.id()),
                }),
                &binding.slot,
            );
        }
        for binding in scalar_slots
            .iter()
            .filter(|binding| !binding.assignment_owned && binding.slot.is_dynamic() == dynamic)
        {
            push_scalar(
                declarations,
                DefaultLocalSearchSelectorFamily::ScalarSwap,
                MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
                    selection_order: Some(SelectionOrder::Random),
                    selection_metric: None,
                    target: target_for(&binding.slot.id()),
                }),
                &binding.slot,
            );
        }
    }
}

pub(super) fn append_group_policy<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    declarations: &mut Vec<DefaultLocalSearchSelectorDeclaration>,
) -> Result<(), RuntimeCompileError>
where
    S: PlanningSolution + 'static,
    V: Clone,
    DM: Clone,
    IDM: Clone,
{
    for group in model.scalar_groups() {
        let config =
            MoveSelectorConfig::GroupedScalarMoveSelector(GroupedScalarMoveSelectorConfig {
                selection_order: Some(SelectionOrder::Random),
                selection_metric: None,
                group_name: group.group_name.to_string(),
                value_candidate_limit: None,
                max_moves_per_step: group.default_max_moves_per_step(),
                require_hard_improvement: false,
            });
        declarations.push(DefaultLocalSearchSelectorDeclaration {
            family: DefaultLocalSearchSelectorFamily::GroupedScalar,
            capability_policy: DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
            config,
            slots: Vec::new(),
        });
    }
    Ok(())
}

pub(super) fn append_conflict_repair_policy<S, V, DM, IDM>(
    model: &RuntimeModel<S, V, DM, IDM>,
    declarations: &mut Vec<DefaultLocalSearchSelectorDeclaration>,
) -> Result<(), RuntimeCompileError>
where
    S: PlanningSolution + 'static,
    V: Clone,
    DM: Clone,
    IDM: Clone,
{
    let registry = model.runtime_provider_registry();
    let mut constraints = registry
        .repairs()
        .iter()
        .flat_map(|binding| binding.declared_constraints.iter())
        .map(|constraint| constraint.to_string())
        .collect::<Vec<_>>();
    constraints.extend(
        registry
            .static_repairs()
            .iter()
            .map(|binding| binding.repair.constraint_name().to_string()),
    );
    constraints.sort();
    constraints.dedup();
    if constraints.is_empty() {
        if model.conflict_repairs().is_empty() {
            return Ok(());
        }
        return Err(default_selector_error(
            "typed conflict repairs require frozen runtime provider bindings before omitted local search can be compiled",
        ));
    }
    let config = MoveSelectorConfig::CompoundConflictRepairMoveSelector(
        CompoundConflictRepairMoveSelectorConfig {
            constraints,
            ..CompoundConflictRepairMoveSelectorConfig::default()
        },
    );
    declarations.push(DefaultLocalSearchSelectorDeclaration {
        family: DefaultLocalSearchSelectorFamily::CompoundConflictRepair,
        capability_policy: DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
        config,
        slots: Vec::new(),
    });
    Ok(())
}

fn push_scalar<S>(
    declarations: &mut Vec<DefaultLocalSearchSelectorDeclaration>,
    family: DefaultLocalSearchSelectorFamily,
    config: MoveSelectorConfig,
    slot: &crate::builder::RuntimeScalarSlot<S>,
) {
    declarations.push(DefaultLocalSearchSelectorDeclaration {
        family,
        capability_policy: DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
        config,
        slots: vec![slot.id()],
    });
}
