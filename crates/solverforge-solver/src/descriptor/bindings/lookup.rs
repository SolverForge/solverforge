use solverforge_core::domain::SolutionDescriptor;

use super::variable::VariableBinding;

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

pub fn descriptor_has_bindings(descriptor: &SolutionDescriptor) -> bool {
    !collect_bindings(descriptor).is_empty()
}
