//! Descriptor-bound runtime-model resolution and provider-registry freezing.

use std::collections::{BTreeMap, HashSet};

use solverforge_core::domain::{DynamicScalarVariableSlot, SolutionDescriptor};

use crate::descriptor::collect_bindings;

use super::{RuntimeModel, RuntimeScalarSlot, VariableSlot};

impl<S, V, DM, IDM> RuntimeModel<S, V, DM, IDM> {
    pub fn resolve_dynamic_descriptor_indexes(
        mut self,
        descriptor: &SolutionDescriptor,
    ) -> Result<Self, String>
    where
        S: 'static,
    {
        for variable in &mut self.variables {
            match variable {
                VariableSlot::DynamicScalar(slot) => slot.resolve_descriptor_index(descriptor)?,
                VariableSlot::DynamicList(slot) => slot.resolve_descriptor_index(descriptor)?,
                VariableSlot::Scalar(_) | VariableSlot::List(_) => {}
            }
        }
        let dynamic_scalar_slots = self.dynamic_scalar_variables().cloned().collect::<Vec<_>>();
        self.canonicalize_dynamic_scalar_groups(&dynamic_scalar_slots)?;
        let provider_scalar_slots = self
            .variables
            .iter()
            .filter_map(|variable| match variable {
                VariableSlot::Scalar(slot) => Some(RuntimeScalarSlot::Static(*slot)),
                VariableSlot::DynamicScalar(slot) => Some(RuntimeScalarSlot::Dynamic(slot.clone())),
                VariableSlot::List(_) | VariableSlot::DynamicList(_) => None,
            })
            .collect::<Vec<_>>();
        self.runtime_provider_registry.freeze(
            &provider_scalar_slots,
            &self.scalar_groups,
            &self.conflict_repairs,
        )?;
        self.bind_scalar_group_construction_slots(descriptor)?;
        Ok(self)
    }

    pub fn assert_dynamic_descriptor_indexes_resolved(&self) {
        for variable in &self.variables {
            match variable {
                VariableSlot::DynamicScalar(slot) => {
                    let _ = slot.descriptor_index();
                    let _ = slot.descriptor_variable_index();
                }
                VariableSlot::DynamicList(slot) => {
                    let _ = slot.descriptor_index();
                    let _ = slot.descriptor_variable_index();
                }
                VariableSlot::Scalar(_) | VariableSlot::List(_) => {}
            }
        }
        for group in &self.scalar_groups {
            for member in &group.members {
                if member.is_dynamic() {
                    let _ = member.dynamic_identity();
                    assert_ne!(
                        member.descriptor_index,
                        usize::MAX,
                        "dynamic scalar group `{}` has an unresolved target descriptor",
                        group.group_name
                    );
                    assert_ne!(
                        member.variable_index,
                        usize::MAX,
                        "dynamic scalar group `{}` has an unresolved target variable",
                        group.group_name
                    );
                }
            }
        }
    }
    fn canonicalize_dynamic_scalar_groups(
        &mut self,
        scalar_slots: &[DynamicScalarVariableSlot<S>],
    ) -> Result<(), String> {
        let mut group_names = HashSet::new();
        let mut assignment_targets = HashSet::new();
        for group in &mut self.scalar_groups {
            if !group_names.insert(group.group_name) {
                return Err(format!(
                    "scalar group `{}` is registered more than once",
                    group.group_name
                ));
            }
            group.canonicalize_dynamic_members(scalar_slots)?;
            if let Some(assignment) = group.assignment() {
                let target = assignment.target();
                if !assignment_targets.insert((target.descriptor_index, target.variable_index)) {
                    return Err(format!(
                        "assignment scalar target {}.{} is owned by more than one scalar group",
                        target.entity_type_name, target.variable_name
                    ));
                }
            }
        }
        Ok(())
    }

    fn bind_scalar_group_construction_slots(
        &mut self,
        descriptor: &SolutionDescriptor,
    ) -> Result<(), String> {
        let typed_binding_indexes = collect_bindings(descriptor)
            .into_iter()
            .map(|binding| {
                (
                    (binding.descriptor_index, binding.variable_index),
                    binding.binding_index,
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut next_dynamic_binding_index = self.variables.len().max(
            typed_binding_indexes
                .values()
                .copied()
                .max()
                .map_or(0, |index| index.saturating_add(1)),
        );
        let mut dynamic_binding_indexes = BTreeMap::new();

        for group in &mut self.scalar_groups {
            for member in &mut group.members {
                let binding_index = if let Some(identity) = member.dynamic_identity() {
                    *dynamic_binding_indexes.entry(identity).or_insert_with(|| {
                        let binding_index = next_dynamic_binding_index;
                        next_dynamic_binding_index = next_dynamic_binding_index
                            .checked_add(1)
                            .expect("dynamic scalar group construction slot index overflow");
                        binding_index
                    })
                } else {
                    *typed_binding_indexes
                        .get(&(member.descriptor_index, member.variable_index))
                        .ok_or_else(|| {
                            format!(
                                "scalar group `{}` target {}.{} has no descriptor construction binding",
                                group.group_name, member.entity_type_name, member.variable_name
                            )
                        })?
                };
                member.set_construction_binding_index(binding_index);
            }

            let assignment_target = group.assignment().map(|assignment| {
                (
                    assignment.target().descriptor_index,
                    assignment.target().variable_index,
                )
            });
            let Some((descriptor_index, variable_index)) = assignment_target else {
                continue;
            };
            let binding_index = group
                .members
                .iter()
                .find(|member| {
                    member.descriptor_index == descriptor_index
                        && member.variable_index == variable_index
                })
                .map(|member| member.construction_binding_index())
                .ok_or_else(|| {
                    format!(
                        "assignment scalar group `{}` target has no registered construction slot",
                        group.group_name
                    )
                })?;
            group
                .assignment_mut()
                .expect("assignment target must retain its scalar group binding")
                .target_mut()
                .set_construction_binding_index(binding_index);
        }

        Ok(())
    }
}
