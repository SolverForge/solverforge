use std::fmt;

use super::value_source::ValueSource;
use super::variable::{ScalarGetter, ScalarSetter, ScalarVariableContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScalarGroupLimits {
    pub value_candidate_limit: Option<usize>,
    pub max_moves_per_step: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScalarGroupEdit {
    pub descriptor_index: usize,
    pub entity_index: usize,
    pub variable_name: &'static str,
    pub to_value: Option<usize>,
}

impl ScalarGroupEdit {
    pub fn set_scalar(
        descriptor_index: usize,
        entity_index: usize,
        variable_name: &'static str,
        to_value: Option<usize>,
    ) -> Self {
        Self {
            descriptor_index,
            entity_index,
            variable_name,
            to_value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScalarGroupCandidate {
    pub reason: &'static str,
    pub edits: Vec<ScalarGroupEdit>,
}

impl ScalarGroupCandidate {
    pub fn new(reason: &'static str, edits: Vec<ScalarGroupEdit>) -> Self {
        Self { reason, edits }
    }
}

pub type ScalarGroupCandidateProvider<S> = fn(&S, ScalarGroupLimits) -> Vec<ScalarGroupCandidate>;

pub struct ScalarGroupMember<S> {
    pub descriptor_index: usize,
    pub variable_index: usize,
    pub entity_type_name: &'static str,
    pub variable_name: &'static str,
    pub getter: ScalarGetter<S>,
    pub setter: ScalarSetter<S>,
    pub value_source: ValueSource<S>,
    pub allows_unassigned: bool,
}

impl<S> Clone for ScalarGroupMember<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ScalarGroupMember<S> {}

impl<S> ScalarGroupMember<S> {
    pub fn from_scalar_context(context: ScalarVariableContext<S>) -> Self {
        Self {
            descriptor_index: context.descriptor_index,
            variable_index: context.variable_index,
            entity_type_name: context.entity_type_name,
            variable_name: context.variable_name,
            getter: context.getter,
            setter: context.setter,
            value_source: context.value_source,
            allows_unassigned: context.allows_unassigned,
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

impl<S> fmt::Debug for ScalarGroupMember<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScalarGroupMember")
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_index", &self.variable_index)
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .field("value_source", &self.value_source)
            .field("allows_unassigned", &self.allows_unassigned)
            .finish()
    }
}

pub struct ScalarGroupContext<S> {
    pub group_name: &'static str,
    pub members: Vec<ScalarGroupMember<S>>,
    pub candidate_provider: ScalarGroupCandidateProvider<S>,
}

impl<S> ScalarGroupContext<S> {
    pub fn new(
        group_name: &'static str,
        members: Vec<ScalarGroupMember<S>>,
        candidate_provider: ScalarGroupCandidateProvider<S>,
    ) -> Self {
        Self {
            group_name,
            members,
            candidate_provider,
        }
    }

    pub fn member_for_edit(&self, edit: &ScalarGroupEdit) -> Option<ScalarGroupMember<S>> {
        self.members.iter().copied().find(|member| {
            member.descriptor_index == edit.descriptor_index
                && member.variable_name == edit.variable_name
        })
    }
}

impl<S> Clone for ScalarGroupContext<S> {
    fn clone(&self) -> Self {
        Self {
            group_name: self.group_name,
            members: self.members.clone(),
            candidate_provider: self.candidate_provider,
        }
    }
}

impl<S> fmt::Debug for ScalarGroupContext<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScalarGroupContext")
            .field("group_name", &self.group_name)
            .field("members", &self.members)
            .finish_non_exhaustive()
    }
}
