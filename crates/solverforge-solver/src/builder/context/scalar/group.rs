use std::fmt;

use crate::planning::{
    ScalarAssignmentDeclaration, ScalarCandidateProvider, ScalarEdit, ScalarGroup, ScalarGroupKind,
    ScalarGroupLimits,
};

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
    pub entity_count: fn(&S) -> usize,
    pub candidate_values: Option<super::variable::ScalarCandidateValues<S>>,
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
            entity_count: slot.entity_count,
            candidate_values: slot.candidate_values,
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

    pub fn entity_count(&self, solution: &S) -> usize {
        (self.entity_count)(solution)
    }

    pub fn candidate_values(
        &self,
        solution: &S,
        entity_index: usize,
        value_candidate_limit: Option<usize>,
    ) -> Vec<usize> {
        if let Some(candidate_values) = self.candidate_values {
            let values = candidate_values(solution, entity_index, self.variable_index);
            return match value_candidate_limit {
                Some(limit) => values.iter().copied().take(limit).collect(),
                None => values.to_vec(),
            };
        }
        match self.value_source {
            ValueSource::Empty => Vec::new(),
            ValueSource::CountableRange { from, to } => {
                let end = value_candidate_limit
                    .map(|limit| from.saturating_add(limit).min(to))
                    .unwrap_or(to);
                (from..end).collect()
            }
            ValueSource::SolutionCount {
                count_fn,
                provider_index,
            } => {
                let count = count_fn(solution, provider_index);
                let end = value_candidate_limit
                    .map(|limit| limit.min(count))
                    .unwrap_or(count);
                (0..end).collect()
            }
            ValueSource::EntitySlice { values_for_entity } => {
                let values = values_for_entity(solution, entity_index, self.variable_index);
                match value_candidate_limit {
                    Some(limit) => values.iter().copied().take(limit).collect(),
                    None => values.to_vec(),
                }
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
    pub kind: ScalarGroupBindingKind<S>,
    pub limits: ScalarGroupLimits,
}

pub enum ScalarGroupBindingKind<S> {
    Candidates {
        candidate_provider: ScalarCandidateProvider<S>,
    },
    Assignment(ScalarAssignmentBinding<S>),
}

impl<S> Clone for ScalarGroupBindingKind<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ScalarGroupBindingKind<S> {}

pub struct ScalarAssignmentBinding<S> {
    pub target: ScalarGroupMemberBinding<S>,
    pub required_entity: Option<fn(&S, usize) -> bool>,
    pub capacity_key: Option<fn(&S, usize, usize) -> Option<usize>>,
    pub position_key: Option<fn(&S, usize) -> i64>,
    pub sequence_key: Option<fn(&S, usize, usize) -> Option<usize>>,
    pub entity_order: Option<fn(&S, usize) -> i64>,
    pub value_order: Option<fn(&S, usize, usize) -> i64>,
}

impl<S> Clone for ScalarAssignmentBinding<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ScalarAssignmentBinding<S> {}

impl<S> ScalarAssignmentBinding<S> {
    fn bind(
        group_name: &'static str,
        members: &[ScalarGroupMemberBinding<S>],
        declaration: ScalarAssignmentDeclaration<S>,
    ) -> Self {
        assert_eq!(
            members.len(),
            1,
            "assignment scalar group `{group_name}` must target exactly one scalar planning variable",
        );
        let target = members[0];
        assert!(
            target.allows_unassigned,
            "assignment scalar group `{group_name}` target {}.{} must allow unassigned values",
            target.entity_type_name, target.variable_name,
        );
        Self {
            target,
            required_entity: declaration.required_entity,
            capacity_key: declaration.capacity_key,
            position_key: declaration.position_key,
            sequence_key: declaration.sequence_key,
            entity_order: declaration.entity_order,
            value_order: declaration.value_order,
        }
    }

    pub fn entity_count(&self, solution: &S) -> usize {
        self.target.entity_count(solution)
    }

    pub fn current_value(&self, solution: &S, entity_index: usize) -> Option<usize> {
        self.target.current_value(solution, entity_index)
    }

    pub fn is_required(&self, solution: &S, entity_index: usize) -> bool {
        self.required_entity
            .map(|required_entity| required_entity(solution, entity_index))
            .unwrap_or(false)
    }

    pub fn capacity_key(&self, solution: &S, entity_index: usize, value: usize) -> Option<usize> {
        self.capacity_key
            .and_then(|capacity_key| capacity_key(solution, entity_index, value))
    }

    pub fn position_key(&self, solution: &S, entity_index: usize) -> Option<i64> {
        self.position_key
            .map(|position_key| position_key(solution, entity_index))
    }

    pub fn sequence_key(&self, solution: &S, entity_index: usize, value: usize) -> Option<usize> {
        self.sequence_key
            .and_then(|sequence_key| sequence_key(solution, entity_index, value))
    }

    pub fn entity_order_key(&self, solution: &S, entity_index: usize) -> Option<i64> {
        self.entity_order
            .map(|entity_order| entity_order(solution, entity_index))
    }

    pub fn value_order_key(&self, solution: &S, entity_index: usize, value: usize) -> Option<i64> {
        self.value_order
            .map(|value_order| value_order(solution, entity_index, value))
    }

    pub fn candidate_values(
        &self,
        solution: &S,
        entity_index: usize,
        value_candidate_limit: Option<usize>,
    ) -> Vec<usize> {
        let mut values =
            self.target
                .candidate_values(solution, entity_index, value_candidate_limit);
        values.sort_by_key(|value| (self.value_order_key(solution, entity_index, *value), *value));
        values
    }

    pub fn value_is_legal(&self, solution: &S, entity_index: usize, value: Option<usize>) -> bool {
        self.target.value_is_legal(solution, entity_index, value)
    }

    pub fn edit(&self, entity_index: usize, value: Option<usize>) -> ScalarEdit<S> {
        ScalarEdit::from_descriptor_index(
            self.target.descriptor_index,
            entity_index,
            self.target.variable_name,
            value,
        )
    }
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
            .collect::<Vec<_>>();

        let kind = match group.kind() {
            ScalarGroupKind::Assignment(declaration) => ScalarGroupBindingKind::Assignment(
                ScalarAssignmentBinding::bind(group.group_name(), &members, declaration),
            ),
            ScalarGroupKind::Candidates { candidate_provider } => {
                ScalarGroupBindingKind::Candidates { candidate_provider }
            }
        };

        Self {
            group_name: group.group_name(),
            members,
            kind,
            limits: group.limits(),
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
            kind: self.kind,
            limits: self.limits,
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
