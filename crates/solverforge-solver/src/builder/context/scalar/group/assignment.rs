use std::sync::Arc;

use solverforge_core::domain::{
    DynamicScalarAssignmentMetadata, DynamicScalarAssignmentMetadataCapabilities,
};

use crate::planning::{ScalarAssignmentDeclaration, ScalarAssignmentRule, ScalarEdit};

use super::member::ScalarGroupMemberBinding;

struct StaticAssignmentMetadata<S> {
    required_entity: Option<fn(&S, usize) -> bool>,
    capacity_key: Option<fn(&S, usize, usize) -> Option<usize>>,
    position_key: Option<fn(&S, usize) -> i64>,
    sequence_key: Option<fn(&S, usize, usize) -> Option<usize>>,
    entity_order: Option<fn(&S, usize) -> i64>,
    value_order: Option<fn(&S, usize, usize) -> i64>,
    assignment_rule: Option<ScalarAssignmentRule<S>>,
}

impl<S> Copy for StaticAssignmentMetadata<S> {}

impl<S> Clone for StaticAssignmentMetadata<S> {
    fn clone(&self) -> Self {
        *self
    }
}

enum AssignmentMetadataAccess<S> {
    Static(StaticAssignmentMetadata<S>),
    Dynamic(Arc<dyn DynamicScalarAssignmentMetadata<S>>),
}

impl<S> Clone for AssignmentMetadataAccess<S> {
    fn clone(&self) -> Self {
        match self {
            Self::Static(metadata) => Self::Static(*metadata),
            Self::Dynamic(metadata) => Self::Dynamic(Arc::clone(metadata)),
        }
    }
}

/// The semantic metadata for one assignment-backed scalar group.
///
/// The access representation is private so grouped construction and search can
/// never distinguish typed hooks from a dynamic binding implementation.
pub struct ScalarAssignmentBinding<S> {
    pub(crate) target: ScalarGroupMemberBinding<S>,
    metadata: AssignmentMetadataAccess<S>,
}

impl<S> Clone for ScalarAssignmentBinding<S> {
    fn clone(&self) -> Self {
        Self {
            target: self.target.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

impl<S> ScalarAssignmentBinding<S> {
    pub(super) fn bind(
        group_name: &'static str,
        members: &[ScalarGroupMemberBinding<S>],
        declaration: ScalarAssignmentDeclaration<S>,
    ) -> Self {
        assert_eq!(
            members.len(),
            1,
            "assignment scalar group `{group_name}` must target exactly one scalar planning variable",
        );
        let target = members[0].clone();
        assert!(
            target.allows_unassigned,
            "assignment scalar group `{group_name}` target {}.{} must allow unassigned values",
            target.entity_type_name, target.variable_name,
        );
        assert!(
            declaration.assignment_rule.is_none() || declaration.sequence_key.is_some(),
            "assignment scalar group `{group_name}` with an assignment rule must declare a sequence key",
        );
        Self {
            target,
            metadata: AssignmentMetadataAccess::Static(StaticAssignmentMetadata {
                required_entity: declaration.required_entity,
                capacity_key: declaration.capacity_key,
                position_key: declaration.position_key,
                sequence_key: declaration.sequence_key,
                entity_order: declaration.entity_order,
                value_order: declaration.value_order,
                assignment_rule: declaration.assignment_rule,
            }),
        }
    }

    pub(super) fn dynamic(
        group_name: &'static str,
        target: ScalarGroupMemberBinding<S>,
        metadata: Arc<dyn DynamicScalarAssignmentMetadata<S>>,
    ) -> Self {
        assert!(
            target.allows_unassigned,
            "assignment scalar group `{group_name}` target {}.{} must allow unassigned values",
            target.entity_type_name, target.variable_name,
        );
        let capabilities = metadata.capabilities();
        assert!(
            !capabilities.assignment_rule || capabilities.sequence_key,
            "assignment scalar group `{group_name}` with an assignment rule must declare a sequence key",
        );
        Self {
            target,
            metadata: AssignmentMetadataAccess::Dynamic(metadata),
        }
    }

    pub(crate) fn target(&self) -> &ScalarGroupMemberBinding<S> {
        &self.target
    }

    pub(crate) fn target_mut(&mut self) -> &mut ScalarGroupMemberBinding<S> {
        &mut self.target
    }

    pub(crate) fn capabilities(&self) -> DynamicScalarAssignmentMetadataCapabilities {
        match &self.metadata {
            AssignmentMetadataAccess::Static(metadata) => {
                DynamicScalarAssignmentMetadataCapabilities {
                    required_entity: metadata.required_entity.is_some(),
                    capacity_key: metadata.capacity_key.is_some(),
                    position_key: metadata.position_key.is_some(),
                    sequence_key: metadata.sequence_key.is_some(),
                    entity_order: metadata.entity_order.is_some(),
                    value_order: metadata.value_order.is_some(),
                    assignment_rule: metadata.assignment_rule.is_some(),
                }
            }
            AssignmentMetadataAccess::Dynamic(metadata) => metadata.capabilities(),
        }
    }

    pub(crate) fn has_entity_order(&self) -> bool {
        self.capabilities().entity_order
    }

    pub(crate) fn has_value_order(&self) -> bool {
        self.capabilities().value_order
    }

    pub(crate) fn has_sequence_metadata(&self) -> bool {
        self.capabilities().sequence_key
    }

    pub(crate) fn has_position_metadata(&self) -> bool {
        self.capabilities().position_key
    }

    pub(crate) fn has_assignment_rule(&self) -> bool {
        self.capabilities().assignment_rule
    }
}

impl<S> ScalarAssignmentBinding<S> {
    pub fn entity_count(&self, solution: &S) -> usize {
        self.target.entity_count(solution)
    }

    pub fn current_value(&self, solution: &S, entity_index: usize) -> Option<usize> {
        self.target.current_value(solution, entity_index)
    }

    pub fn is_required(&self, solution: &S, entity_index: usize) -> bool {
        match &self.metadata {
            AssignmentMetadataAccess::Static(metadata) => metadata
                .required_entity
                .map(|required_entity| required_entity(solution, entity_index))
                .unwrap_or(false),
            AssignmentMetadataAccess::Dynamic(metadata) => {
                metadata.required_entity(solution, entity_index)
            }
        }
    }

    pub fn capacity_key(&self, solution: &S, entity_index: usize, value: usize) -> Option<usize> {
        match &self.metadata {
            AssignmentMetadataAccess::Static(metadata) => metadata
                .capacity_key
                .and_then(|capacity_key| capacity_key(solution, entity_index, value)),
            AssignmentMetadataAccess::Dynamic(metadata) => {
                metadata.capacity_key(solution, entity_index, value)
            }
        }
    }

    pub fn position_key(&self, solution: &S, entity_index: usize) -> Option<i64> {
        match &self.metadata {
            AssignmentMetadataAccess::Static(metadata) => metadata
                .position_key
                .map(|position_key| position_key(solution, entity_index)),
            AssignmentMetadataAccess::Dynamic(metadata) => {
                metadata.position_key(solution, entity_index)
            }
        }
    }

    pub fn sequence_key(&self, solution: &S, entity_index: usize, value: usize) -> Option<usize> {
        match &self.metadata {
            AssignmentMetadataAccess::Static(metadata) => metadata
                .sequence_key
                .and_then(|sequence_key| sequence_key(solution, entity_index, value)),
            AssignmentMetadataAccess::Dynamic(metadata) => {
                metadata.sequence_key(solution, entity_index, value)
            }
        }
    }

    pub fn entity_order_key(&self, solution: &S, entity_index: usize) -> Option<i64> {
        match &self.metadata {
            AssignmentMetadataAccess::Static(metadata) => metadata
                .entity_order
                .map(|entity_order| entity_order(solution, entity_index)),
            AssignmentMetadataAccess::Dynamic(metadata) => {
                metadata.entity_order_key(solution, entity_index)
            }
        }
    }

    pub fn value_order_key(&self, solution: &S, entity_index: usize, value: usize) -> Option<i64> {
        match &self.metadata {
            AssignmentMetadataAccess::Static(metadata) => metadata
                .value_order
                .map(|value_order| value_order(solution, entity_index, value)),
            AssignmentMetadataAccess::Dynamic(metadata) => {
                metadata.value_order_key(solution, entity_index, value)
            }
        }
    }

    pub fn assignment_edge_allowed(
        &self,
        solution: &S,
        left_entity: usize,
        left_value: usize,
        right_entity: usize,
        right_value: usize,
    ) -> bool {
        match &self.metadata {
            AssignmentMetadataAccess::Static(metadata) => metadata
                .assignment_rule
                .map(|assignment_rule| {
                    assignment_rule(solution, left_entity, left_value, right_entity, right_value)
                })
                .unwrap_or(true),
            AssignmentMetadataAccess::Dynamic(metadata) => metadata.assignment_edge_allowed(
                solution,
                left_entity,
                left_value,
                right_entity,
                right_value,
            ),
        }
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

    pub fn remaining_required_count(&self, solution: &S) -> u64 {
        (0..self.entity_count(solution))
            .filter(|entity_index| {
                self.is_required(solution, *entity_index)
                    && self.current_value(solution, *entity_index).is_none()
            })
            .fold(0_u64, |count, _| count.saturating_add(1))
    }

    pub fn unassigned_count(&self, solution: &S) -> u64 {
        (0..self.entity_count(solution))
            .filter(|entity_index| self.current_value(solution, *entity_index).is_none())
            .fold(0_u64, |count, _| count.saturating_add(1))
    }
}
