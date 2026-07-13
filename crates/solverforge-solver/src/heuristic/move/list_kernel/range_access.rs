//! Range-mutation access traits for shared reverse and permutation moves.

use std::fmt;

use solverforge_core::domain::DynamicListVariableSlot;

use crate::builder::context::{
    list_access::{ListAccess, ListAccessError},
    RuntimeListSlot,
};
use crate::builder::ListVariableSlot;
use crate::heuristic::r#move::metadata::encode_option_debug;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

use super::access::runtime_list_tabu_value_id;

pub(crate) trait ListRangeAccess<S> {
    type Element: Clone + Send + Sync + fmt::Debug + 'static;

    fn descriptor_index(&self) -> usize;
    fn variable_name(&self) -> &'static str;
    fn list_len(&self, solution: &S, entity: usize) -> usize;
    fn list_get(&self, solution: &S, entity: usize, position: usize) -> Option<Self::Element>;

    /// Canonical value identity for tabu metadata; see `ListMoveAccess`.
    fn tabu_value_id(&self, _solution: &S, value: Option<&Self::Element>) -> u64 {
        encode_option_debug(value)
    }
}

pub(crate) trait ListReverseAccess<S>: ListRangeAccess<S> {
    fn list_reverse(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<(), ListAccessError>;
}

pub(crate) trait ListWindowAccess<S>: ListRangeAccess<S> {
    fn sublist_remove(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<Vec<Self::Element>, ListAccessError>;
    fn sublist_insert(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
        values: Vec<Self::Element>,
    ) -> Result<(), ListAccessError>;
}

impl<S, V, DM, IDM> ListRangeAccess<S> for ListVariableSlot<S, V, DM, IDM>
where
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Element = V;

    fn descriptor_index(&self) -> usize {
        ListAccess::descriptor_index(self)
    }

    fn variable_name(&self) -> &'static str {
        ListAccess::variable_name(self)
    }

    fn list_len(&self, solution: &S, entity: usize) -> usize {
        ListAccess::list_len(self, solution, entity)
    }

    fn list_get(&self, solution: &S, entity: usize, position: usize) -> Option<Self::Element> {
        ListAccess::list_get(self, solution, entity, position)
    }
}

impl<S, V, DM, IDM> ListReverseAccess<S> for ListVariableSlot<S, V, DM, IDM>
where
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn list_reverse(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<(), ListAccessError> {
        ListAccess::list_reverse(self, solution, entity, start, end)
    }
}

impl<S, V, DM, IDM> ListWindowAccess<S> for ListVariableSlot<S, V, DM, IDM>
where
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn sublist_remove(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<Vec<Self::Element>, ListAccessError> {
        ListAccess::sublist_remove(self, solution, entity, start, end)
    }

    fn sublist_insert(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
        values: Vec<Self::Element>,
    ) -> Result<(), ListAccessError> {
        ListAccess::sublist_insert(self, solution, entity, position, values)
    }
}

impl<S> ListRangeAccess<S> for DynamicListVariableSlot<S>
where
    S: Clone + Send + Sync + 'static,
{
    type Element = usize;

    fn descriptor_index(&self) -> usize {
        ListAccess::descriptor_index(self)
    }

    fn variable_name(&self) -> &'static str {
        ListAccess::variable_name(self)
    }

    fn list_len(&self, solution: &S, entity: usize) -> usize {
        ListAccess::list_len(self, solution, entity)
    }

    fn list_get(&self, solution: &S, entity: usize, position: usize) -> Option<Self::Element> {
        ListAccess::list_get(self, solution, entity, position)
    }
}

impl<S> ListReverseAccess<S> for DynamicListVariableSlot<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn list_reverse(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<(), ListAccessError> {
        ListAccess::list_reverse(self, solution, entity, start, end)
    }
}

impl<S> ListWindowAccess<S> for DynamicListVariableSlot<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn sublist_remove(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<Vec<Self::Element>, ListAccessError> {
        ListAccess::sublist_remove(self, solution, entity, start, end)
    }

    fn sublist_insert(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
        values: Vec<Self::Element>,
    ) -> Result<(), ListAccessError> {
        ListAccess::sublist_insert(self, solution, entity, position, values)
    }
}

impl<S, V, DM, IDM> ListRangeAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Element = crate::builder::context::RuntimeListElement<V>;

    fn descriptor_index(&self) -> usize {
        ListAccess::descriptor_index(self)
    }

    fn variable_name(&self) -> &'static str {
        ListAccess::variable_name(self)
    }

    fn list_len(&self, solution: &S, entity: usize) -> usize {
        ListAccess::list_len(self, solution, entity)
    }

    fn list_get(&self, solution: &S, entity: usize, position: usize) -> Option<Self::Element> {
        ListAccess::list_get(self, solution, entity, position)
    }

    fn tabu_value_id(&self, solution: &S, value: Option<&Self::Element>) -> u64 {
        runtime_list_tabu_value_id(self, solution, value)
    }
}

impl<S, V, DM, IDM> ListReverseAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn list_reverse(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<(), ListAccessError> {
        ListAccess::list_reverse(self, solution, entity, start, end)
    }
}

impl<S, V, DM, IDM> ListWindowAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn sublist_remove(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<Vec<Self::Element>, ListAccessError> {
        ListAccess::sublist_remove(self, solution, entity, start, end)
    }

    fn sublist_insert(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
        values: Vec<Self::Element>,
    ) -> Result<(), ListAccessError> {
        ListAccess::sublist_insert(self, solution, entity, position, values)
    }
}
