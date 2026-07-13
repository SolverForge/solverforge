use std::fmt;
use std::sync::Arc;

use solverforge_core::domain::{DynamicScalarAssignmentMetadata, DynamicScalarVariableSlot};

use crate::planning::{ScalarCandidateProvider, ScalarGroup, ScalarGroupKind, ScalarGroupLimits};

use super::variable::ScalarVariableSlot;

mod assignment;
mod member;

pub use assignment::ScalarAssignmentBinding;
pub use member::ScalarGroupMemberBinding;

pub struct ScalarGroupBinding<S> {
    pub group_name: &'static str,
    pub members: Vec<ScalarGroupMemberBinding<S>>,
    pub kind: ScalarGroupBindingKind<S>,
    pub limits: ScalarGroupLimits,
}

// Keep assignment metadata inline so the canonical grouped construction and
// local-search path remains allocation-free after model compilation.
#[allow(clippy::large_enum_variant)]
pub enum ScalarGroupBindingKind<S> {
    Candidates {
        candidate_provider: ScalarCandidateProvider<S>,
    },
    Assignment(ScalarAssignmentBinding<S>),
}

impl<S> Clone for ScalarGroupBindingKind<S> {
    fn clone(&self) -> Self {
        match self {
            Self::Candidates { candidate_provider } => Self::Candidates {
                candidate_provider: *candidate_provider,
            },
            Self::Assignment(assignment) => Self::Assignment(assignment.clone()),
        }
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

    pub fn dynamic_assignment(
        group_name: &'static str,
        target: DynamicScalarVariableSlot<S>,
        metadata: Arc<dyn DynamicScalarAssignmentMetadata<S>>,
        limits: ScalarGroupLimits,
    ) -> Self {
        let member = ScalarGroupMemberBinding::from_dynamic_slot(target);
        let assignment = ScalarAssignmentBinding::dynamic(group_name, member.clone(), metadata);
        Self {
            group_name,
            members: vec![member],
            kind: ScalarGroupBindingKind::Assignment(assignment),
            limits,
        }
    }

    pub fn member_for_edit(
        &self,
        edit: &crate::planning::ScalarEdit<S>,
    ) -> Option<ScalarGroupMemberBinding<S>> {
        self.members
            .iter()
            .find(|member| {
                member.descriptor_index == edit.descriptor_index()
                    && member.variable_name == edit.variable_name()
            })
            .cloned()
    }

    pub fn assignment(&self) -> Option<&ScalarAssignmentBinding<S>> {
        match &self.kind {
            ScalarGroupBindingKind::Assignment(assignment) => Some(assignment),
            ScalarGroupBindingKind::Candidates { .. } => None,
        }
    }

    pub(crate) fn assignment_mut(&mut self) -> Option<&mut ScalarAssignmentBinding<S>> {
        match &mut self.kind {
            ScalarGroupBindingKind::Assignment(assignment) => Some(assignment),
            ScalarGroupBindingKind::Candidates { .. } => None,
        }
    }

    pub(crate) fn canonicalize_dynamic_members(
        &mut self,
        scalar_slots: &[DynamicScalarVariableSlot<S>],
    ) -> Result<(), String> {
        for member in &mut self.members {
            let Some((entity, variable)) = member.dynamic_identity() else {
                continue;
            };
            let slot = scalar_slots
                .iter()
                .find(|slot| slot.entity == entity && slot.variable == variable)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "dynamic scalar group `{}` targets unregistered scalar variable {}.{}",
                        self.group_name, member.entity_type_name, member.variable_name
                    )
                })?;
            member.canonicalize_dynamic_slot(slot)?;
        }

        let group_name = self.group_name;
        let Some(assignment) = self.assignment_mut() else {
            return Ok(());
        };
        let Some((entity, variable)) = assignment.target().dynamic_identity() else {
            return Ok(());
        };
        let slot = scalar_slots
            .iter()
            .find(|slot| slot.entity == entity && slot.variable == variable)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "dynamic scalar group `{}` targets an unregistered assignment variable",
                    group_name
                )
            })?;
        assignment.target_mut().canonicalize_dynamic_slot(slot)
    }

    pub fn is_assignment(&self) -> bool {
        matches!(self.kind, ScalarGroupBindingKind::Assignment(_))
    }

    pub fn is_candidate_group(&self) -> bool {
        matches!(self.kind, ScalarGroupBindingKind::Candidates { .. })
    }

    pub fn has_sequence_metadata(&self) -> bool {
        self.assignment()
            .is_some_and(ScalarAssignmentBinding::has_sequence_metadata)
    }

    pub fn has_position_metadata(&self) -> bool {
        self.assignment()
            .is_some_and(ScalarAssignmentBinding::has_position_metadata)
    }

    pub fn default_max_moves_per_step(&self) -> Option<usize> {
        self.limits.max_moves_per_step
    }
}

impl<S> Clone for ScalarGroupBinding<S> {
    fn clone(&self) -> Self {
        Self {
            group_name: self.group_name,
            members: self.members.clone(),
            kind: self.kind.clone(),
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
