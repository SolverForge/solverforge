use std::any::Any;
use std::fmt::{self, Debug};
use std::ops::Deref;

use solverforge_core::domain::{
    PlanningSolution, SolutionDescriptor, UsizeConstructionEntityOrderKey,
    UsizeConstructionValueOrderKey, UsizeEntityValueProvider, UsizeGetter, UsizeSetter,
    ValueRangeType,
};

use crate::builder::context::{ConstructionEntityOrderKey, ConstructionValueOrderKey};
use crate::phase::construction::{ConstructionFrontier, ConstructionSlotId};

#[derive(Clone)]
pub(crate) struct VariableBinding {
    pub(crate) binding_index: usize,
    pub(crate) descriptor_index: usize,
    pub(crate) variable_index: usize,
    pub(crate) entity_type_name: &'static str,
    pub(crate) variable_name: &'static str,
    pub(crate) allows_unassigned: bool,
    pub(crate) getter: UsizeGetter,
    pub(crate) setter: UsizeSetter,
    pub(crate) value_range_provider: Option<&'static str>,
    pub(crate) provider: Option<UsizeEntityValueProvider>,
    pub(crate) range_type: ValueRangeType,
    pub(crate) nearby_value_distance_meter:
        Option<solverforge_core::domain::UsizeNearbyValueDistanceMeter>,
    pub(crate) nearby_entity_distance_meter:
        Option<solverforge_core::domain::UsizeNearbyEntityDistanceMeter>,
    pub(crate) construction_entity_order_key: Option<UsizeConstructionEntityOrderKey>,
    pub(crate) construction_value_order_key: Option<UsizeConstructionValueOrderKey>,
}

impl Debug for VariableBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VariableBinding")
            .field("binding_index", &self.binding_index)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_index", &self.variable_index)
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

    pub(crate) fn entity_for_index<'a>(
        &self,
        solution_descriptor: &SolutionDescriptor,
        solution: &'a dyn Any,
        entity_index: usize,
    ) -> &'a dyn Any {
        solution_descriptor
            .get_entity(solution, self.descriptor_index, entity_index)
            .expect("entity lookup failed for descriptor scalar binding")
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
                .solution_value_count(solution_descriptor, solution)
                .map(|count| (0..count).collect())
                .unwrap_or_default(),
        }
    }

    pub(crate) fn values_for_entity_index(
        &self,
        solution_descriptor: &SolutionDescriptor,
        solution: &dyn Any,
        entity_index: usize,
    ) -> Vec<usize> {
        let entity = self.entity_for_index(solution_descriptor, solution, entity_index);
        self.values_for_entity(solution_descriptor, solution, entity)
    }

    pub(crate) fn has_values_for_entity_index(
        &self,
        solution_descriptor: &SolutionDescriptor,
        solution: &dyn Any,
        entity_index: usize,
    ) -> bool {
        let entity = self.entity_for_index(solution_descriptor, solution, entity_index);
        match (&self.provider, &self.range_type) {
            (Some(provider), _) => !provider(entity).is_empty(),
            (_, ValueRangeType::CountableRange { from, to }) => from < to,
            _ => self
                .solution_value_count(solution_descriptor, solution)
                .is_some_and(|count| count > 0),
        }
    }

    pub(crate) fn solution_value_count(
        &self,
        solution_descriptor: &SolutionDescriptor,
        solution: &dyn Any,
    ) -> Option<usize> {
        self.value_range_provider.and_then(|provider_name| {
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
    }

    pub(crate) fn has_unspecified_value_range(&self) -> bool {
        self.provider.is_none()
            && self.value_range_provider.is_none()
            && !matches!(self.range_type, ValueRangeType::CountableRange { .. })
    }

    pub(crate) fn countable_range_contains(from: i64, to: i64, value: usize) -> bool {
        i64::try_from(value).is_ok_and(|value| from <= value && value < to)
    }

    pub(crate) fn value_is_legal_for_entity_index(
        &self,
        solution_descriptor: &SolutionDescriptor,
        solution: &dyn Any,
        entity_index: usize,
        candidate: Option<usize>,
    ) -> bool {
        let entity = self.entity_for_index(solution_descriptor, solution, entity_index);
        self.value_is_legal_for_entity(solution_descriptor, solution, entity, candidate)
    }

    pub(crate) fn value_is_legal_for_entity(
        &self,
        solution_descriptor: &SolutionDescriptor,
        solution: &dyn Any,
        entity: &dyn Any,
        candidate: Option<usize>,
    ) -> bool {
        let Some(value) = candidate else {
            return self.allows_unassigned;
        };
        match (&self.provider, &self.range_type) {
            (Some(provider), _) => provider(entity).into_iter().any(|allowed| allowed == value),
            (_, ValueRangeType::CountableRange { from, to }) => {
                Self::countable_range_contains(*from, *to, value)
            }
            _ => self
                .solution_value_count(solution_descriptor, solution)
                .is_some_and(|count| value < count),
        }
    }

    pub(crate) fn entity_order_key(&self, solution: &dyn Any, entity_index: usize) -> Option<i64> {
        self.construction_entity_order_key
            .map(|order_key| order_key(solution, entity_index))
    }

    pub(crate) fn value_order_key(
        &self,
        solution: &dyn Any,
        entity_index: usize,
        value: usize,
    ) -> Option<i64> {
        self.construction_value_order_key
            .map(|order_key| order_key(solution, entity_index, value))
    }
}

pub(crate) struct ResolvedVariableBinding<S> {
    binding: VariableBinding,
    runtime_construction_entity_order_key: Option<ConstructionEntityOrderKey<S>>,
    runtime_construction_value_order_key: Option<ConstructionValueOrderKey<S>>,
}

impl<S> Clone for ResolvedVariableBinding<S> {
    fn clone(&self) -> Self {
        Self {
            binding: self.binding.clone(),
            runtime_construction_entity_order_key: self.runtime_construction_entity_order_key,
            runtime_construction_value_order_key: self.runtime_construction_value_order_key,
        }
    }
}

impl<S> Debug for ResolvedVariableBinding<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolvedVariableBinding")
            .field("binding", &self.binding)
            .field(
                "has_runtime_construction_entity_order_key",
                &self.runtime_construction_entity_order_key.is_some(),
            )
            .field(
                "has_runtime_construction_value_order_key",
                &self.runtime_construction_value_order_key.is_some(),
            )
            .finish()
    }
}

impl<S> Deref for ResolvedVariableBinding<S> {
    type Target = VariableBinding;

    fn deref(&self) -> &Self::Target {
        &self.binding
    }
}

impl<S> ResolvedVariableBinding<S> {
    pub(crate) fn new(binding: VariableBinding) -> Self {
        Self {
            binding,
            runtime_construction_entity_order_key: None,
            runtime_construction_value_order_key: None,
        }
    }

    pub(crate) fn with_runtime_construction_hooks(
        mut self,
        entity_order_key: Option<ConstructionEntityOrderKey<S>>,
        value_order_key: Option<ConstructionValueOrderKey<S>>,
    ) -> Self {
        self.runtime_construction_entity_order_key = entity_order_key;
        self.runtime_construction_value_order_key = value_order_key;
        self
    }

    pub(crate) fn has_entity_order_key(&self) -> bool {
        self.runtime_construction_entity_order_key.is_some()
            || self.binding.construction_entity_order_key.is_some()
    }

    pub(crate) fn has_value_order_key(&self) -> bool {
        self.runtime_construction_value_order_key.is_some()
            || self.binding.construction_value_order_key.is_some()
    }

    pub(crate) fn runtime_value_order_key(&self) -> Option<ConstructionValueOrderKey<S>> {
        self.runtime_construction_value_order_key
    }

    pub(crate) fn clone_binding(&self) -> VariableBinding {
        self.binding.clone()
    }
}

impl<S> ResolvedVariableBinding<S>
where
    S: PlanningSolution + 'static,
{
    pub(crate) fn entity_order_key(&self, solution: &S, entity_index: usize) -> Option<i64> {
        self.runtime_construction_entity_order_key
            .and_then(|order_key| order_key(solution, entity_index, self.binding.variable_index))
            .or_else(|| {
                self.binding
                    .entity_order_key(solution as &dyn Any, entity_index)
            })
    }

    pub(crate) fn value_order_key(
        &self,
        solution: &S,
        entity_index: usize,
        value: usize,
    ) -> Option<i64> {
        self.runtime_construction_value_order_key
            .and_then(|order_key| {
                order_key(solution, entity_index, self.binding.variable_index, value)
            })
            .or_else(|| {
                self.binding
                    .value_order_key(solution as &dyn Any, entity_index, value)
            })
    }
}

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
