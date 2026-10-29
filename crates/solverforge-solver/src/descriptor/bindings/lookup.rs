use solverforge_core::domain::SolutionDescriptor;

use super::variable::VariableBinding;

pub(crate) fn collect_bindings(descriptor: &SolutionDescriptor) -> Vec<VariableBinding> {
    let mut bindings = Vec::new();
    for (descriptor_index, entity_descriptor) in descriptor.entity_descriptors.iter().enumerate() {
        let mut variable_index = 0usize;
        for variable in entity_descriptor.genuine_variable_descriptors() {
            // Only variables with typed `usize` access are scalar bindings.
            if variable.usize_getter.is_none() || variable.usize_setter.is_none() {
                continue;
            }
            bindings.push(VariableBinding {
                binding_index: bindings.len(),
                descriptor_index,
                variable_index,
                entity_type_name: entity_descriptor.type_name,
                variable_name: variable.name,
                allows_unassigned: variable.allows_unassigned,
                range_type: variable.value_range_type.clone(),
            });
            variable_index += 1;
        }
    }
    bindings
}
