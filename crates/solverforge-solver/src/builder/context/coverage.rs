use std::fmt;

use crate::builder::context::ScalarVariableSlot;
use crate::planning::{CoverageGroup, CoverageGroupLimits, ScalarEdit};

pub struct CoverageGroupBinding<S> {
    pub group_name: &'static str,
    pub target: ScalarVariableSlot<S>,
    pub required_slot: fn(&S, usize) -> bool,
    pub capacity_key: Option<fn(&S, usize, usize) -> Option<usize>>,
    pub entity_order: Option<fn(&S, usize) -> i64>,
    pub value_order: Option<fn(&S, usize, usize) -> i64>,
    pub limits: CoverageGroupLimits,
}

impl<S> Clone for CoverageGroupBinding<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for CoverageGroupBinding<S> {}

impl<S> CoverageGroupBinding<S> {
    pub fn bind(group: CoverageGroup<S>, scalar_slots: &[ScalarVariableSlot<S>]) -> Self {
        let target = group.target();
        let target_slot = scalar_slots
            .iter()
            .copied()
            .find(|slot| {
                slot.descriptor_index == target.descriptor_index()
                    && slot.variable_name == target.variable_name()
            })
            .unwrap_or_else(|| {
                panic!(
                    "coverage group `{}` target {}.{} did not match a scalar planning variable",
                    group.group_name(),
                    target.descriptor_index(),
                    target.variable_name(),
                )
            });
        assert!(
            target_slot.allows_unassigned,
            "coverage group `{}` target {}.{} must allow unassigned values",
            group.group_name(),
            target_slot.entity_type_name,
            target_slot.variable_name,
        );
        let required_slot = group.required_slot().unwrap_or_else(|| {
            panic!(
                "coverage group `{}` requires a required-slot predicate",
                group.group_name(),
            )
        });
        Self {
            group_name: group.group_name(),
            target: target_slot,
            required_slot,
            capacity_key: group.capacity_key(),
            entity_order: group.entity_order(),
            value_order: group.value_order(),
            limits: group.limits(),
        }
    }

    pub fn entity_count(&self, solution: &S) -> usize {
        (self.target.entity_count)(solution)
    }

    pub fn current_value(&self, solution: &S, entity_index: usize) -> Option<usize> {
        self.target.current_value(solution, entity_index)
    }

    pub fn is_required(&self, solution: &S, entity_index: usize) -> bool {
        (self.required_slot)(solution, entity_index)
    }

    pub fn capacity_key(&self, solution: &S, entity_index: usize, value: usize) -> Option<usize> {
        self.capacity_key
            .and_then(|capacity_key| capacity_key(solution, entity_index, value))
    }

    pub fn entity_order_key(&self, solution: &S, entity_index: usize) -> i64 {
        self.entity_order
            .map(|entity_order| entity_order(solution, entity_index))
            .unwrap_or(entity_index as i64)
    }

    pub fn value_order_key(&self, solution: &S, entity_index: usize, value: usize) -> i64 {
        self.value_order
            .map(|value_order| value_order(solution, entity_index, value))
            .unwrap_or(value as i64)
    }

    pub fn candidate_values(
        &self,
        solution: &S,
        entity_index: usize,
        value_candidate_limit: Option<usize>,
    ) -> Vec<usize> {
        let mut values =
            self.target
                .candidate_values_for_entity(solution, entity_index, value_candidate_limit);
        values.sort_by_key(|value| self.value_order_key(solution, entity_index, *value));
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

impl<S> fmt::Debug for CoverageGroupBinding<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CoverageGroupBinding")
            .field("group_name", &self.group_name)
            .field("entity_type_name", &self.target.entity_type_name)
            .field("variable_name", &self.target.variable_name)
            .field("has_capacity_key", &self.capacity_key.is_some())
            .field("limits", &self.limits)
            .finish()
    }
}

pub fn bind_coverage_groups<S>(
    groups: Vec<CoverageGroup<S>>,
    scalar_slots: &[ScalarVariableSlot<S>],
) -> Vec<CoverageGroupBinding<S>> {
    groups
        .into_iter()
        .map(|group| CoverageGroupBinding::bind(group, scalar_slots))
        .collect()
}
