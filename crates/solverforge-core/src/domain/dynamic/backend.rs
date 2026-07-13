use crate::domain::{EntityClassId, VariableId};

use super::{DynamicListAccess, DynamicModelBackend, DynamicScalarAccess};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BackendScalarAccess {
    pub(super) entity: EntityClassId,
    pub(super) variable: VariableId,
}

impl<S> DynamicScalarAccess<S> for BackendScalarAccess
where
    S: DynamicModelBackend,
{
    fn entity_class(&self) -> EntityClassId {
        self.entity
    }

    fn variable(&self) -> VariableId {
        self.variable
    }

    fn entity_count(&self, solution: &S) -> usize {
        solution.entity_count(self.entity)
    }

    fn get(&self, solution: &S, row: usize) -> Option<usize> {
        solution.get_scalar(self.entity, row, self.variable)
    }

    fn set(&self, solution: &mut S, row: usize, value: Option<usize>) {
        solution.set_scalar(self.entity, row, self.variable, value);
    }

    fn candidate_values<'a>(&self, solution: &'a S, row: usize) -> &'a [usize] {
        solution.candidate_values(self.entity, row, self.variable)
    }

    fn value_is_legal(&self, solution: &S, row: usize, value: usize) -> bool {
        solution.scalar_value_is_legal(self.entity, row, self.variable, value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BackendListAccess {
    pub(super) entity: EntityClassId,
    pub(super) variable: VariableId,
}

impl<S> DynamicListAccess<S> for BackendListAccess
where
    S: DynamicModelBackend,
{
    fn entity_class(&self) -> EntityClassId {
        self.entity
    }

    fn variable(&self) -> VariableId {
        self.variable
    }

    fn entity_count(&self, solution: &S) -> usize {
        solution.entity_count(self.entity)
    }

    fn element_count(&self, solution: &S) -> usize {
        solution.list_element_count(self.entity, self.variable)
    }

    fn element(&self, solution: &S, element_index: usize) -> Option<usize> {
        solution.list_element(self.entity, self.variable, element_index)
    }

    fn assigned_elements(&self, solution: &S) -> Vec<usize> {
        solution.list_assigned_elements(self.entity, self.variable)
    }

    fn len(&self, solution: &S, row: usize) -> usize {
        solution.list_len(self.entity, row, self.variable)
    }

    fn get(&self, solution: &S, row: usize, pos: usize) -> Option<usize> {
        solution.list_get(self.entity, row, self.variable, pos)
    }

    fn insert(&self, solution: &mut S, row: usize, pos: usize, value: usize) {
        solution.list_insert(self.entity, row, self.variable, pos, value);
    }

    fn remove(&self, solution: &mut S, row: usize, pos: usize) -> Option<usize> {
        solution.list_remove(self.entity, row, self.variable, pos)
    }
}
