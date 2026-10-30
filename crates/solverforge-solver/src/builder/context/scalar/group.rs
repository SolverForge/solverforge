use std::fmt;
use std::sync::Arc;

use solverforge_core::domain::{DynamicScalarAssignmentMetadata, DynamicScalarVariableSlot};

use crate::planning::{ScalarCandidateProvider, ScalarGroup, ScalarGroupKind, ScalarGroupLimits};

use super::variable::ScalarVariableSlot;

pub(crate) mod assignment;
pub(crate) mod member;

pub(crate) use assignment::ScalarAssignmentBinding;
pub(crate) use member::ScalarGroupMemberBinding;

/// A [`ScalarGroup`](crate::ScalarGroup) bound to the scalar variables of a
/// model.
pub struct ScalarGroupBinding<S> {
    /// Name of the group.
    pub group_name: &'static str,
    pub(crate) members: Vec<ScalarGroupMemberBinding<S>>,
    pub(crate) kind: ScalarGroupBindingKind<S>,
    /// Limits declared on the group.
    pub limits: ScalarGroupLimits,
}

/// Keep assignment metadata inline so the canonical grouped construction and
/// local-search path remains allocation-free after model compilation.
#[allow(clippy::large_enum_variant)]
pub(crate) enum ScalarGroupBindingKind<S> {
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
    /// Binds `group` to the scalar slots its targets name.
    ///
    /// # Panics
    ///
    /// Panics when a target names no slot in `scalar_slots`, and for an
    /// assignment group whose declaration is invalid (more than one target, a
    /// target that does not allow unassigned values, or an assignment rule
    /// without a sequence key).
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

    /// Builds an assignment group over a dynamic scalar `target` whose
    /// assignment metadata comes from a host binding.
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

    pub(crate) fn member_for_edit(
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

    pub(crate) fn assignment(&self) -> Option<&ScalarAssignmentBinding<S>> {
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

    /// Whether this is an assignment-backed group.
    pub fn is_assignment(&self) -> bool {
        matches!(self.kind, ScalarGroupBindingKind::Assignment(_))
    }

    /// Whether this is a candidate-backed group.
    pub fn is_candidate_group(&self) -> bool {
        matches!(self.kind, ScalarGroupBindingKind::Candidates { .. })
    }

    /// Whether the assignment declares a sequence key.
    pub fn has_sequence_metadata(&self) -> bool {
        self.assignment()
            .is_some_and(ScalarAssignmentBinding::has_sequence_metadata)
    }

    /// Whether the assignment declares a position key.
    pub fn has_position_metadata(&self) -> bool {
        self.assignment()
            .is_some_and(ScalarAssignmentBinding::has_position_metadata)
    }

    /// The group's per-step move limit; `None` means no limit.
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

/// Binds every group with [`ScalarGroupBinding::bind`].
///
/// # Panics
///
/// As [`ScalarGroupBinding::bind`].
pub fn bind_scalar_groups<S>(
    groups: Vec<ScalarGroup<S>>,
    scalar_slots: &[ScalarVariableSlot<S>],
) -> Vec<ScalarGroupBinding<S>> {
    groups
        .into_iter()
        .map(|group| ScalarGroupBinding::bind(group, scalar_slots))
        .collect()
}
