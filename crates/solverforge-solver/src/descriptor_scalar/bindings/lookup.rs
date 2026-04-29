use std::any::Any;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use crate::phase::construction::ConstructionFrontier;

use super::variable::{ResolvedVariableBinding, VariableBinding};

pub(crate) fn collect_bindings(descriptor: &SolutionDescriptor) -> Vec<VariableBinding> {
    let mut bindings = Vec::new();
    for (descriptor_index, entity_descriptor) in descriptor.entity_descriptors.iter().enumerate() {
        let mut variable_index = 0usize;
        for variable in entity_descriptor.genuine_variable_descriptors() {
            let Some(getter) = variable.usize_getter else {
                continue;
            };
            let Some(setter) = variable.usize_setter else {
                continue;
            };
            bindings.push(VariableBinding {
                binding_index: bindings.len(),
                descriptor_index,
                variable_index,
                entity_type_name: entity_descriptor.type_name,
                variable_name: variable.name,
                allows_unassigned: variable.allows_unassigned,
                getter,
                setter,
                value_range_provider: variable.value_range_provider,
                provider: variable.entity_value_provider,
                candidate_values: variable.candidate_values,
                nearby_value_candidates: variable.nearby_value_candidates,
                nearby_entity_candidates: variable.nearby_entity_candidates,
                range_type: variable.value_range_type.clone(),
                nearby_value_distance_meter: variable.nearby_value_distance_meter,
                nearby_entity_distance_meter: variable.nearby_entity_distance_meter,
                construction_entity_order_key: variable.construction_entity_order_key,
                construction_value_order_key: variable.construction_value_order_key,
            });
            variable_index += 1;
        }
    }
    bindings
}

pub(crate) fn find_binding(
    bindings: &[VariableBinding],
    entity_class: Option<&str>,
    variable_name: Option<&str>,
) -> Vec<VariableBinding> {
    bindings
        .iter()
        .filter(|binding| entity_class.is_none_or(|name| name == binding.entity_type_name))
        .filter(|binding| variable_name.is_none_or(|name| name == binding.variable_name))
        .cloned()
        .collect()
}

pub(crate) fn find_resolved_binding<S>(
    bindings: &[ResolvedVariableBinding<S>],
    entity_class: Option<&str>,
    variable_name: Option<&str>,
) -> Vec<ResolvedVariableBinding<S>> {
    bindings
        .iter()
        .filter(|binding| entity_class.is_none_or(|name| name == binding.entity_type_name))
        .filter(|binding| variable_name.is_none_or(|name| name == binding.variable_name))
        .cloned()
        .collect()
}

pub fn descriptor_has_bindings(descriptor: &SolutionDescriptor) -> bool {
    !collect_bindings(descriptor).is_empty()
}

pub(crate) fn scalar_work_remaining_with_frontier<S>(
    descriptor: &SolutionDescriptor,
    frontier: &ConstructionFrontier,
    solution_revision: u64,
    entity_class: Option<&str>,
    variable_name: Option<&str>,
    solution: &S,
) -> bool
where
    S: PlanningSolution + 'static,
{
    let bindings = find_binding(&collect_bindings(descriptor), entity_class, variable_name);
    for binding in bindings {
        let Some(entity_count) = descriptor
            .entity_descriptors
            .get(binding.descriptor_index)
            .and_then(|entity| entity.entity_count(solution as &dyn Any))
        else {
            continue;
        };
        for entity_index in 0..entity_count {
            let entity = descriptor
                .get_entity(solution as &dyn Any, binding.descriptor_index, entity_index)
                .expect("entity lookup failed while checking scalar work");
            if (binding.getter)(entity).is_some()
                || frontier.is_scalar_completed(binding.slot_id(entity_index), solution_revision)
            {
                continue;
            }
            if !binding
                .values_for_entity(descriptor, solution as &dyn Any, entity)
                .is_empty()
            {
                return true;
            }
        }
    }
    false
}

pub fn scalar_work_remaining<S>(
    descriptor: &SolutionDescriptor,
    entity_class: Option<&str>,
    variable_name: Option<&str>,
    solution: &S,
) -> bool
where
    S: PlanningSolution + 'static,
{
    let bindings = find_binding(&collect_bindings(descriptor), entity_class, variable_name);
    for binding in bindings {
        let Some(entity_count) = descriptor
            .entity_descriptors
            .get(binding.descriptor_index)
            .and_then(|entity| entity.entity_count(solution as &dyn Any))
        else {
            continue;
        };
        for entity_index in 0..entity_count {
            let entity = descriptor
                .get_entity(solution as &dyn Any, binding.descriptor_index, entity_index)
                .expect("entity lookup failed while checking scalar work");
            if (binding.getter)(entity).is_none()
                && !binding
                    .values_for_entity(descriptor, solution as &dyn Any, entity)
                    .is_empty()
            {
                return true;
            }
        }
    }
    false
}

pub fn scalar_target_matches(
    descriptor: &SolutionDescriptor,
    entity_class: Option<&str>,
    variable_name: Option<&str>,
) -> bool {
    !find_binding(&collect_bindings(descriptor), entity_class, variable_name).is_empty()
}
