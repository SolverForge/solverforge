use crate::domain::{EntityClassId, SolutionDescriptor, VariableId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DynamicVariableKind {
    Scalar,
    List,
}

pub(super) fn resolve_dynamic_descriptor_index(
    descriptor: &SolutionDescriptor,
    entity: EntityClassId,
    variable: VariableId,
    entity_type_name: &'static str,
    variable_name: &'static str,
    kind: DynamicVariableKind,
) -> Result<usize, String> {
    let Some(descriptor_index) = descriptor.entity_descriptor_index_by_logical_id(entity) else {
        return Err(format!(
            "dynamic variable {entity_type_name}.{variable_name} refers to logical entity ID {}, but the solution descriptor has no matching entity descriptor",
            entity.0
        ));
    };
    let entity_descriptor = descriptor
        .entity_descriptors
        .get(descriptor_index)
        .expect("logical entity index must point at an entity descriptor");
    if entity_descriptor.type_name != entity_type_name {
        return Err(format!(
            "dynamic variable {entity_type_name}.{variable_name} resolved logical entity ID {} to descriptor index {descriptor_index} ({})",
            entity.0, entity_descriptor.type_name
        ));
    }

    let Some(variable_descriptor) = entity_descriptor
        .variable_descriptors
        .iter()
        .find(|descriptor| descriptor.logical_id == Some(variable))
    else {
        return Err(format!(
            "dynamic variable {entity_type_name}.{variable_name} refers to logical variable ID {}, but descriptor {entity_type_name} has no matching variable descriptor",
            variable.0
        ));
    };
    if variable_descriptor.name != variable_name {
        return Err(format!(
            "dynamic variable {entity_type_name}.{variable_name} resolved logical variable ID {} to descriptor variable {}",
            variable.0, variable_descriptor.name
        ));
    }
    match kind {
        DynamicVariableKind::Scalar if variable_descriptor.variable_type.is_list() => Err(
            format!(
                "dynamic scalar variable {entity_type_name}.{variable_name} resolves to a list variable descriptor"
            ),
        ),
        DynamicVariableKind::List if !variable_descriptor.variable_type.is_list() => Err(
            format!(
                "dynamic list variable {entity_type_name}.{variable_name} resolves to a non-list variable descriptor"
            ),
        ),
        DynamicVariableKind::Scalar | DynamicVariableKind::List => Ok(descriptor_index),
    }
}
