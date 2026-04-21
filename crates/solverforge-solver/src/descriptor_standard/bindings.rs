use std::any::Any;
use std::fmt::{self, Debug};

use solverforge_core::domain::{
    PlanningSolution, SolutionDescriptor, UsizeEntityValueProvider, UsizeGetter, UsizeSetter,
    ValueRangeType,
};

use crate::phase::construction::ConstructionSlotId;

use super::ConstructionFrontier;

#[derive(Clone)]
pub(crate) struct VariableBinding {
    pub(crate) binding_index: usize,
    pub(crate) descriptor_index: usize,
    pub(crate) entity_type_name: &'static str,
    pub(crate) variable_name: &'static str,
    pub(crate) allows_unassigned: bool,
    pub(crate) getter: UsizeGetter,
    pub(crate) setter: UsizeSetter,
    pub(crate) value_range_provider: Option<&'static str>,
    pub(crate) provider: Option<UsizeEntityValueProvider>,
    pub(crate) range_type: ValueRangeType,
}

impl Debug for VariableBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VariableBinding")
            .field("binding_index", &self.binding_index)
            .field("descriptor_index", &self.descriptor_index)
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .field("allows_unassigned", &self.allows_unassigned)
            .field("range_type", &self.range_type)
            .finish()
    }
}

impl VariableBinding {
    pub(crate) fn slot_id(&self, entity_index: usize) -> ConstructionSlotId {
        ConstructionSlotId::new(self.binding_index, entity_index)
    }

    pub(crate) fn values_for_entity(
        &self,
        solution_descriptor: &SolutionDescriptor,
        solution: &dyn Any,
        entity: &dyn Any,
    ) -> Vec<usize> {
        match (&self.provider, &self.range_type) {
            (Some(provider), _) => provider(entity),
            (_, ValueRangeType::CountableRange { from, to }) => {
                let start = *from;
                let end = *to;
                (start..end)
                    .filter_map(|value| usize::try_from(value).ok())
                    .collect()
            }
            _ => self
                .value_range_provider
                .and_then(|provider_name| {
                    solution_descriptor
                        .problem_fact_descriptors
                        .iter()
                        .find(|descriptor| descriptor.solution_field == provider_name)
                        .and_then(|descriptor| descriptor.extractor.as_ref())
                        .and_then(|extractor| extractor.count(solution))
                        .or_else(|| {
                            solution_descriptor
                                .entity_descriptors
                                .iter()
                                .find(|descriptor| descriptor.solution_field == provider_name)
                                .and_then(|descriptor| descriptor.extractor.as_ref())
                                .and_then(|extractor| extractor.count(solution))
                        })
                })
                .map(|count| (0..count).collect())
                .unwrap_or_default(),
        }
    }
}

pub(crate) fn collect_bindings(descriptor: &SolutionDescriptor) -> Vec<VariableBinding> {
    let mut bindings = Vec::new();
    for (descriptor_index, entity_descriptor) in descriptor.entity_descriptors.iter().enumerate() {
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
                entity_type_name: entity_descriptor.type_name,
                variable_name: variable.name,
                allows_unassigned: variable.allows_unassigned,
                getter,
                setter,
                value_range_provider: variable.value_range_provider,
                provider: variable.entity_value_provider,
                range_type: variable.value_range_type.clone(),
            });
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

pub(crate) fn standard_work_remaining_with_frontier<S>(
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
                .expect("entity lookup failed while checking standard work");
            if (binding.getter)(entity).is_some()
                || frontier.is_completed(binding.slot_id(entity_index), solution_revision)
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

pub fn standard_work_remaining<S>(
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
                .expect("entity lookup failed while checking standard work");
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

pub fn standard_target_matches(
    descriptor: &SolutionDescriptor,
    entity_class: Option<&str>,
    variable_name: Option<&str>,
) -> bool {
    !find_binding(&collect_bindings(descriptor), entity_class, variable_name).is_empty()
}
