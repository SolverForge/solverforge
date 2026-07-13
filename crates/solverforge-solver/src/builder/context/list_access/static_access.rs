use std::fmt;

use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

use super::super::ListVariableSlot;
use super::{ListAccess, ListAccessCapability, ListAccessError};

pub(super) fn typed_error<S, V, DM, IDM>(
    slot: &ListVariableSlot<S, V, DM, IDM>,
    capability: ListAccessCapability,
) -> ListAccessError {
    ListAccessError {
        capability,
        entity_type_name: slot.entity_type_name,
        variable_name: slot.variable_name,
    }
}

impl<S, V, DM, IDM> ListAccess<S> for ListVariableSlot<S, V, DM, IDM>
where
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Element = V;

    fn entity_type_name(&self) -> &'static str {
        self.entity_type_name
    }

    fn variable_name(&self) -> &'static str {
        self.variable_name
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_count(&self, solution: &S) -> usize {
        (self.entity_count)(solution)
    }

    fn element_count(&self, solution: &S) -> usize {
        (self.element_count)(solution)
    }

    fn index_to_element(&self, solution: &S, element_index: usize) -> Option<Self::Element> {
        Some((self.index_to_element)(solution, element_index))
    }

    fn element_source_key(&self, solution: &S, element: &Self::Element) -> usize {
        (self.element_source_key)(solution, element)
    }

    fn assigned_elements(&self, solution: &S) -> Vec<Self::Element> {
        (self.assigned_elements)(solution)
    }

    fn list_len(&self, solution: &S, entity: usize) -> usize {
        (self.list_len)(solution, entity)
    }

    fn list_get(&self, solution: &S, entity: usize, pos: usize) -> Option<Self::Element> {
        (self.list_get)(solution, entity, pos)
    }

    fn list_insert(&self, solution: &mut S, entity: usize, pos: usize, value: Self::Element) {
        (self.list_insert)(solution, entity, pos, value);
    }

    fn list_remove(&self, solution: &mut S, entity: usize, pos: usize) -> Option<Self::Element> {
        (self.list_remove)(solution, entity, pos)
    }

    fn list_set(
        &self,
        solution: &mut S,
        entity: usize,
        pos: usize,
        value: Self::Element,
    ) -> Result<(), ListAccessError> {
        (self.list_set)(solution, entity, pos, value);
        Ok(())
    }

    fn list_reverse(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<(), ListAccessError> {
        (self.list_reverse)(solution, entity, start, end);
        Ok(())
    }

    fn sublist_remove(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<Vec<Self::Element>, ListAccessError> {
        Ok((self.sublist_remove)(solution, entity, start, end))
    }

    fn sublist_insert(
        &self,
        solution: &mut S,
        entity: usize,
        pos: usize,
        values: Vec<Self::Element>,
    ) -> Result<(), ListAccessError> {
        (self.sublist_insert)(solution, entity, pos, values);
        Ok(())
    }

    fn element_owner(
        &self,
        solution: &S,
        element: &Self::Element,
    ) -> Result<Option<usize>, ListAccessError> {
        self.element_owner_fn
            .map(|owner| owner(solution, element))
            .ok_or_else(|| typed_error(self, ListAccessCapability::ElementOwner))
    }

    fn construction_order_key(
        &self,
        solution: &S,
        element: Self::Element,
    ) -> Result<i64, ListAccessError> {
        self.construction_element_order_key
            .map(|key| key(solution, element))
            .ok_or_else(|| typed_error(self, ListAccessCapability::ConstructionOrderKey))
    }

    fn extend_precedence_successors(
        &self,
        solution: &S,
        element: Self::Element,
        successors: &mut Vec<Self::Element>,
    ) -> Result<(), ListAccessError> {
        let Some(successors_fn) = self.precedence_successors_fn else {
            return Err(typed_error(self, ListAccessCapability::Precedence));
        };
        successors_fn(solution, element, successors);
        Ok(())
    }

    fn precedence_duration(
        &self,
        solution: &S,
        element: Self::Element,
    ) -> Result<usize, ListAccessError> {
        self.precedence_duration_fn
            .map(|duration| duration(solution, element))
            .ok_or_else(|| typed_error(self, ListAccessCapability::Precedence))
    }

    fn cross_position_distance(
        &self,
        solution: &S,
        from_entity: usize,
        from_position: usize,
        to_entity: usize,
        to_position: usize,
    ) -> Result<f64, ListAccessError> {
        Ok(self.cross_distance_meter.distance(
            solution,
            from_entity,
            from_position,
            to_entity,
            to_position,
        ))
    }

    fn intra_position_distance(
        &self,
        solution: &S,
        entity: usize,
        from_position: usize,
        to_position: usize,
    ) -> Result<f64, ListAccessError> {
        Ok(self
            .intra_distance_meter
            .distance(solution, entity, from_position, entity, to_position))
    }
}
