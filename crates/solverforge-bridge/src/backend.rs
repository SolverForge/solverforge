//! Dynamic planning-model access traits.

use solverforge_core::score::Score;

use crate::{EntityClassId, VariableId};

/// Rust-owned dynamic planning model backend.
///
/// Binding crates implement this trait on their concrete dynamic solution
/// state. The trait is deliberately expressed in logical entity and variable
/// IDs rather than Rust `TypeId`s.
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
}

/// Object-safe dynamic scalar variable access.
///
/// This is the public shape the solver runtime needs to consume directly for a
/// fully dynamic binding path. Existing static `fn` slots remain valid for the
/// macro path.
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
///
/// List access includes the variable ID because a dynamic solution may have
/// more than one list variable backed by the same Rust state type.
pub trait DynamicListAccess<S>: Send + Sync
where
    S: Clone + Send + Sync + 'static,
{
    fn entity_class(&self) -> EntityClassId;
    fn variable(&self) -> VariableId;
    fn entity_count(&self, solution: &S) -> usize;
    fn element_count(&self, solution: &S) -> usize;
    fn assigned_elements(&self, solution: &S) -> Vec<usize>;
    fn len(&self, solution: &S, row: usize) -> usize;
    fn get(&self, solution: &S, row: usize, pos: usize) -> Option<usize>;
    fn insert(&self, solution: &mut S, row: usize, pos: usize, value: usize);
    fn remove(&self, solution: &mut S, row: usize, pos: usize) -> Option<usize>;
}
