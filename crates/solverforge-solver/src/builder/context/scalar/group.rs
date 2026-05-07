use std::fmt;

use crate::planning::{ScalarCandidateProvider, ScalarEdit, ScalarGroup};

use super::value_source::ValueSource;
use super::variable::{ScalarGetter, ScalarSetter, ScalarVariableSlot};

pub struct ScalarGroupMemberBinding<S> {
    pub descriptor_index: usize,
    pub variable_index: usize,
    pub entity_type_name: &'static str,
    pub variable_name: &'static str,
    pub getter: ScalarGetter<S>,
    pub setter: ScalarSetter<S>,
    pub value_source: ValueSource<S>,
    pub allows_unassigned: bool,
}

impl<S> Clone for ScalarGroupMemberBinding<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ScalarGroupMemberBinding<S> {}

impl<S> ScalarGroupMemberBinding<S> {
    pub fn from_scalar_slot(slot: ScalarVariableSlot<S>) -> Self {
        Self {
            descriptor_index: slot.descriptor_index,
            variable_index: slot.variable_index,
            entity_type_name: slot.entity_type_name,
            variable_name: slot.variable_name,
            getter: slot.getter,
            setter: slot.setter,
            value_source: slot.value_source,
            allows_unassigned: slot.allows_unassigned,
        }
    }

    pub fn current_value(&self, solution: &S, entity_index: usize) -> Option<usize> {
        (self.getter)(solution, entity_index, self.variable_index)
    }

    pub fn value_is_legal(
        &self,
        solution: &S,
        entity_index: usize,
        candidate: Option<usize>,
    ) -> bool {
        let Some(value) = candidate else {
            return self.allows_unassigned;
        };
        match self.value_source {
            ValueSource::Empty => false,
            ValueSource::CountableRange { from, to } => from <= value && value < to,
            ValueSource::SolutionCount {
                count_fn,
                provider_index,
            } => value < count_fn(solution, provider_index),
            ValueSource::EntitySlice { values_for_entity } => {
                values_for_entity(solution, entity_index, self.variable_index).contains(&value)
            }
        }
    }
}

impl<S> fmt::Debug for ScalarGroupMemberBinding<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScalarGroupMemberBinding")
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_index", &self.variable_index)
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .field("value_source", &self.value_source)
            .field("allows_unassigned", &self.allows_unassigned)
            .finish()
    }
}

pub struct ScalarGroupBinding<S> {
    pub group_name: &'static str,
    pub members: Vec<ScalarGroupMemberBinding<S>>,
    pub candidate_provider: ScalarCandidateProvider<S>,
}

impl<S> ScalarGroupBinding<S> {
    pub fn bind(group: ScalarGroup<S>, scalar_slots: &[ScalarVariableSlot<S>]) -> Self {
        let members = group
            .targets()
            .iter()
            .map(|target| {
                let descriptor_index = target.descriptor_index();
                let variable_name = target.variable_name();
                let slot = scalar_slots
                    .iter()
                    .copied()
                    .find(|slot| {
                        slot.descriptor_index == descriptor_index
                            && slot.variable_name == variable_name
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "scalar group `{}` targets unknown scalar variable `{}` on descriptor {}",
                            group.group_name(),
                            variable_name,
                            descriptor_index
                        )
                    });
                ScalarGroupMemberBinding::from_scalar_slot(slot)
            })
            .collect();

        Self {
            group_name: group.group_name(),
            members,
            candidate_provider: group.candidate_provider(),
        }
    }

    pub fn member_for_edit(&self, edit: &ScalarEdit<S>) -> Option<ScalarGroupMemberBinding<S>> {
        self.members.iter().copied().find(|member| {
            member.descriptor_index == edit.descriptor_index()
                && member.variable_name == edit.variable_name()
        })
    }
}

impl<S> Clone for ScalarGroupBinding<S> {
    fn clone(&self) -> Self {
        Self {
            group_name: self.group_name,
            members: self.members.clone(),
            candidate_provider: self.candidate_provider,
        }
    }
}

impl<S> fmt::Debug for ScalarGroupBinding<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScalarGroupBinding")
            .field("group_name", &self.group_name)
            .field("members", &self.members)
            .finish_non_exhaustive()
    }
}

pub fn bind_scalar_groups<S>(
    groups: Vec<ScalarGroup<S>>,
    scalar_slots: &[ScalarVariableSlot<S>],
) -> Vec<ScalarGroupBinding<S>> {
    groups
        .into_iter()
        .map(|group| ScalarGroupBinding::bind(group, scalar_slots))
        .collect()
}
