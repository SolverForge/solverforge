//! Dynamic model contracts for host-language integrations.

use std::fmt;
use std::sync::Arc;

use crate::domain::{EntityClassId, VariableId};
use crate::score::Score;

/// Rust-owned dynamic planning model backend.
///
/// Binding crates implement this trait on their concrete dynamic solution
/// state. The trait is expressed in logical entity and variable IDs rather
/// than Rust `TypeId`s.
pub trait DynamicModelBackend: Clone + Send + Sync + 'static {
    type Score: Score;

    fn entity_count(&self, entity: EntityClassId) -> usize;

    fn get_scalar(&self, entity: EntityClassId, row: usize, variable: VariableId) -> Option<usize>;

    fn set_scalar(
        &mut self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        value: Option<usize>,
    );

    fn list_len(&self, entity: EntityClassId, row: usize, variable: VariableId) -> usize;

    fn list_get(
        &self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        pos: usize,
    ) -> Option<usize>;

    fn list_insert(
        &mut self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        pos: usize,
        value: usize,
    );

    fn list_remove(
        &mut self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        pos: usize,
    ) -> Option<usize>;

    fn candidate_values(&self, entity: EntityClassId, row: usize, variable: VariableId)
        -> &[usize];

    fn list_element_count(&self, _entity: EntityClassId, _variable: VariableId) -> usize {
        0
    }

    fn list_element(
        &self,
        _entity: EntityClassId,
        _variable: VariableId,
        element_index: usize,
    ) -> Option<usize> {
        Some(element_index)
    }

    fn list_assigned_elements(&self, _entity: EntityClassId, _variable: VariableId) -> Vec<usize> {
        Vec::new()
    }
}

/// Object-safe dynamic scalar variable access.
pub trait DynamicScalarAccess<S>: Send + Sync
where
    S: Clone + Send + Sync + 'static,
{
    fn entity_class(&self) -> EntityClassId;
    fn variable(&self) -> VariableId;
    fn entity_count(&self, solution: &S) -> usize;
    fn get(&self, solution: &S, row: usize) -> Option<usize>;
    fn set(&self, solution: &mut S, row: usize, value: Option<usize>);
    fn candidate_values<'a>(&self, solution: &'a S, row: usize) -> &'a [usize];
}

/// Object-safe dynamic list variable access.
pub trait DynamicListAccess<S>: Send + Sync
where
    S: Clone + Send + Sync + 'static,
{
    fn entity_class(&self) -> EntityClassId;
    fn variable(&self) -> VariableId;
    fn entity_count(&self, solution: &S) -> usize;
    fn element_count(&self, solution: &S) -> usize;
    fn element(&self, solution: &S, element_index: usize) -> Option<usize>;
    fn assigned_elements(&self, solution: &S) -> Vec<usize>;
    fn len(&self, solution: &S, row: usize) -> usize;
    fn get(&self, solution: &S, row: usize, pos: usize) -> Option<usize>;
    fn insert(&self, solution: &mut S, row: usize, pos: usize, value: usize);
    fn remove(&self, solution: &mut S, row: usize, pos: usize) -> Option<usize>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BackendScalarAccess {
    entity: EntityClassId,
    variable: VariableId,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BackendListAccess {
    entity: EntityClassId,
    variable: VariableId,
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

/// Public dynamic scalar variable slot.
pub struct DynamicScalarVariableSlot<S> {
    pub entity: EntityClassId,
    pub variable: VariableId,
    pub entity_type_name: &'static str,
    pub variable_name: &'static str,
    pub allows_unassigned: bool,
    access: Arc<dyn DynamicScalarAccess<S>>,
}

impl<S> Clone for DynamicScalarVariableSlot<S> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            variable: self.variable,
            entity_type_name: self.entity_type_name,
            variable_name: self.variable_name,
            allows_unassigned: self.allows_unassigned,
            access: Arc::clone(&self.access),
        }
    }
}

impl<S> fmt::Debug for DynamicScalarVariableSlot<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicScalarVariableSlot")
            .field("entity", &self.entity)
            .field("variable", &self.variable)
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .field("allows_unassigned", &self.allows_unassigned)
            .finish()
    }
}

impl<S> PartialEq for DynamicScalarVariableSlot<S> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
            && self.variable == other.variable
            && self.entity_type_name == other.entity_type_name
            && self.variable_name == other.variable_name
            && self.allows_unassigned == other.allows_unassigned
    }
}

impl<S> Eq for DynamicScalarVariableSlot<S> {}

impl<S> DynamicScalarVariableSlot<S> {
    pub fn with_access(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
        allows_unassigned: bool,
        access: Arc<dyn DynamicScalarAccess<S>>,
    ) -> Self {
        Self {
            entity,
            variable,
            entity_type_name,
            variable_name,
            allows_unassigned,
            access,
        }
    }

    pub fn descriptor_index(&self) -> usize {
        self.entity.0
    }

    pub fn matches_target(&self, entity_class: Option<&str>, variable_name: Option<&str>) -> bool {
        entity_class.is_none_or(|entity| entity == self.entity_type_name)
            && variable_name.is_none_or(|variable| variable == self.variable_name)
    }

    pub fn entity_count(&self, solution: &S) -> usize
    where
        S: Clone + Send + Sync + 'static,
    {
        self.access.entity_count(solution)
    }

    pub fn current_value(&self, solution: &S, row: usize) -> Option<usize>
    where
        S: Clone + Send + Sync + 'static,
    {
        self.access.get(solution, row)
    }

    pub fn set_value(&self, solution: &mut S, row: usize, value: Option<usize>)
    where
        S: Clone + Send + Sync + 'static,
    {
        self.access.set(solution, row, value);
    }

    pub fn candidate_values<'a>(&self, solution: &'a S, row: usize) -> &'a [usize]
    where
        S: Clone + Send + Sync + 'static,
    {
        self.access.candidate_values(solution, row)
    }

    pub fn value_is_legal(&self, solution: &S, row: usize, value: Option<usize>) -> bool
    where
        S: Clone + Send + Sync + 'static,
    {
        let Some(value) = value else {
            return self.allows_unassigned;
        };
        self.candidate_values(solution, row).contains(&value)
    }
}

impl<S> DynamicScalarVariableSlot<S>
where
    S: DynamicModelBackend,
{
    pub fn new(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
        allows_unassigned: bool,
    ) -> Self {
        Self::with_access(
            entity,
            variable,
            entity_type_name,
            variable_name,
            allows_unassigned,
            Arc::new(BackendScalarAccess { entity, variable }),
        )
    }
}

/// Public dynamic list variable slot.
pub struct DynamicListVariableSlot<S> {
    pub entity: EntityClassId,
    pub variable: VariableId,
    pub entity_type_name: &'static str,
    pub variable_name: &'static str,
    access: Arc<dyn DynamicListAccess<S>>,
}

impl<S> Clone for DynamicListVariableSlot<S> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            variable: self.variable,
            entity_type_name: self.entity_type_name,
            variable_name: self.variable_name,
            access: Arc::clone(&self.access),
        }
    }
}

impl<S> fmt::Debug for DynamicListVariableSlot<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicListVariableSlot")
            .field("entity", &self.entity)
            .field("variable", &self.variable)
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S> PartialEq for DynamicListVariableSlot<S> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
            && self.variable == other.variable
            && self.entity_type_name == other.entity_type_name
            && self.variable_name == other.variable_name
    }
}

impl<S> Eq for DynamicListVariableSlot<S> {}

impl<S> DynamicListVariableSlot<S> {
    pub fn with_access(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
        access: Arc<dyn DynamicListAccess<S>>,
    ) -> Self {
        Self {
            entity,
            variable,
            entity_type_name,
            variable_name,
            access,
        }
    }

    pub fn descriptor_index(&self) -> usize {
        self.entity.0
    }

    pub fn matches_target(&self, entity_class: Option<&str>, variable_name: Option<&str>) -> bool {
        entity_class.is_none_or(|entity| entity == self.entity_type_name)
            && variable_name.is_none_or(|variable| variable == self.variable_name)
    }

    pub fn entity_count(&self, solution: &S) -> usize
    where
        S: Clone + Send + Sync + 'static,
    {
        self.access.entity_count(solution)
    }

    pub fn element_count(&self, solution: &S) -> usize
    where
        S: Clone + Send + Sync + 'static,
    {
        self.access.element_count(solution)
    }

    pub fn element(&self, solution: &S, element_index: usize) -> Option<usize>
    where
        S: Clone + Send + Sync + 'static,
    {
        self.access.element(solution, element_index)
    }

    pub fn assigned_elements(&self, solution: &S) -> Vec<usize>
    where
        S: Clone + Send + Sync + 'static,
    {
        self.access.assigned_elements(solution)
    }

    pub fn list_len(&self, solution: &S, row: usize) -> usize
    where
        S: Clone + Send + Sync + 'static,
    {
        self.access.len(solution, row)
    }

    pub fn list_get(&self, solution: &S, row: usize, pos: usize) -> Option<usize>
    where
        S: Clone + Send + Sync + 'static,
    {
        self.access.get(solution, row, pos)
    }

    pub fn list_insert(&self, solution: &mut S, row: usize, pos: usize, value: usize)
    where
        S: Clone + Send + Sync + 'static,
    {
        self.access.insert(solution, row, pos, value);
    }

    pub fn list_remove(&self, solution: &mut S, row: usize, pos: usize) -> Option<usize>
    where
        S: Clone + Send + Sync + 'static,
    {
        self.access.remove(solution, row, pos)
    }
}

impl<S> DynamicListVariableSlot<S>
where
    S: DynamicModelBackend,
{
    pub fn new(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
    ) -> Self {
        Self::with_access(
            entity,
            variable,
            entity_type_name,
            variable_name,
            Arc::new(BackendListAccess { entity, variable }),
        )
    }
}
