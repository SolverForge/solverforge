//! Dynamic model contracts for host-language integrations.

use std::fmt;
use std::sync::Arc;

use crate::domain::{EntityClassId, SolutionDescriptor, VariableId};
use crate::score::Score;

mod backend;
mod resolution;

use backend::{BackendListAccess, BackendScalarAccess};
use resolution::{resolve_dynamic_descriptor_index, DynamicVariableKind};

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

/// Public dynamic scalar variable slot.
pub struct DynamicScalarVariableSlot<S> {
    pub entity: EntityClassId,
    pub variable: VariableId,
    pub entity_type_name: &'static str,
    pub variable_name: &'static str,
    pub allows_unassigned: bool,
    descriptor_index: Option<usize>,
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
            descriptor_index: self.descriptor_index,
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
            .field("descriptor_index", &self.descriptor_index)
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
            && self.descriptor_index == other.descriptor_index
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
            descriptor_index: None,
            access,
        }
    }

    pub fn with_descriptor_index(mut self, descriptor_index: usize) -> Self {
        self.descriptor_index = Some(descriptor_index);
        self
    }

    pub fn resolve_descriptor_index(
        &mut self,
        descriptor: &SolutionDescriptor,
    ) -> Result<(), String> {
        let descriptor_index = resolve_dynamic_descriptor_index(
            descriptor,
            self.entity,
            self.variable,
            self.entity_type_name,
            self.variable_name,
            DynamicVariableKind::Scalar,
        )?;
        if let Some(existing) = self.descriptor_index {
            if existing != descriptor_index {
                return Err(format!(
                    "dynamic scalar variable {}.{} was pre-bound to descriptor index {existing}, but logical entity ID {} resolves to descriptor index {descriptor_index}",
                    self.entity_type_name, self.variable_name, self.entity.0
                ));
            }
        }
        self.descriptor_index = Some(descriptor_index);
        Ok(())
    }

    pub fn resolved_against(mut self, descriptor: &SolutionDescriptor) -> Result<Self, String> {
        self.resolve_descriptor_index(descriptor)?;
        Ok(self)
    }

    pub fn is_descriptor_resolved(&self) -> bool {
        self.descriptor_index.is_some()
    }

    pub fn descriptor_index(&self) -> usize {
        self.descriptor_index.unwrap_or_else(|| {
            panic!(
                "dynamic scalar variable {}.{} has not been resolved against a SolutionDescriptor",
                self.entity_type_name, self.variable_name
            )
        })
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
    descriptor_index: Option<usize>,
    access: Arc<dyn DynamicListAccess<S>>,
}

impl<S> Clone for DynamicListVariableSlot<S> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            variable: self.variable,
            entity_type_name: self.entity_type_name,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
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
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S> PartialEq for DynamicListVariableSlot<S> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
            && self.variable == other.variable
            && self.entity_type_name == other.entity_type_name
            && self.variable_name == other.variable_name
            && self.descriptor_index == other.descriptor_index
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
            descriptor_index: None,
            access,
        }
    }

    pub fn with_descriptor_index(mut self, descriptor_index: usize) -> Self {
        self.descriptor_index = Some(descriptor_index);
        self
    }

    pub fn resolve_descriptor_index(
        &mut self,
        descriptor: &SolutionDescriptor,
    ) -> Result<(), String> {
        let descriptor_index = resolve_dynamic_descriptor_index(
            descriptor,
            self.entity,
            self.variable,
            self.entity_type_name,
            self.variable_name,
            DynamicVariableKind::List,
        )?;
        if let Some(existing) = self.descriptor_index {
            if existing != descriptor_index {
                return Err(format!(
                    "dynamic list variable {}.{} was pre-bound to descriptor index {existing}, but logical entity ID {} resolves to descriptor index {descriptor_index}",
                    self.entity_type_name, self.variable_name, self.entity.0
                ));
            }
        }
        self.descriptor_index = Some(descriptor_index);
        Ok(())
    }

    pub fn resolved_against(mut self, descriptor: &SolutionDescriptor) -> Result<Self, String> {
        self.resolve_descriptor_index(descriptor)?;
        Ok(self)
    }

    pub fn is_descriptor_resolved(&self) -> bool {
        self.descriptor_index.is_some()
    }

    pub fn descriptor_index(&self) -> usize {
        self.descriptor_index.unwrap_or_else(|| {
            panic!(
                "dynamic list variable {}.{} has not been resolved against a SolutionDescriptor",
                self.entity_type_name, self.variable_name
            )
        })
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
