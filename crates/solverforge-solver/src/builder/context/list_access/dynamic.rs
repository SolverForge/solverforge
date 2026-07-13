use solverforge_core::domain::{
    DynamicListAccessCapabilities, DynamicListMetadataCapabilities, DynamicListVariableSlot,
};

use super::{ListAccess, ListAccessCapability, ListAccessError};

pub(super) fn dynamic_error<S>(
    slot: &DynamicListVariableSlot<S>,
    capability: ListAccessCapability,
) -> ListAccessError {
    ListAccessError {
        capability,
        entity_type_name: slot.entity_type_name,
        variable_name: slot.variable_name,
    }
}

pub(super) fn dynamic_access_has(
    capabilities: DynamicListAccessCapabilities,
    capability: ListAccessCapability,
) -> bool {
    match capability {
        ListAccessCapability::Set => capabilities.set,
        ListAccessCapability::Replace => capabilities.replace,
        ListAccessCapability::Reverse => capabilities.reverse,
        ListAccessCapability::Sublist => capabilities.sublist,
        ListAccessCapability::ElementOwner
        | ListAccessCapability::ConstructionOrderKey
        | ListAccessCapability::Precedence
        | ListAccessCapability::CrossPositionDistance
        | ListAccessCapability::IntraPositionDistance
        | ListAccessCapability::Route
        | ListAccessCapability::Savings => false,
    }
}

pub(super) fn dynamic_metadata_has(
    capabilities: DynamicListMetadataCapabilities,
    capability: ListAccessCapability,
) -> bool {
    match capability {
        ListAccessCapability::ElementOwner => capabilities.element_owner,
        ListAccessCapability::ConstructionOrderKey => capabilities.construction_order_key,
        // Compiler-level precedence consumers require the complete pair.
        // Individual accessors below deliberately validate their own half so
        // successor-only ListRuin metadata remains executable.
        ListAccessCapability::Precedence => {
            capabilities.precedence_duration && capabilities.precedence_successors
        }
        ListAccessCapability::CrossPositionDistance => capabilities.cross_position_distance,
        ListAccessCapability::IntraPositionDistance => capabilities.intra_position_distance,
        ListAccessCapability::Route => capabilities.route,
        ListAccessCapability::Savings => capabilities.savings,
        ListAccessCapability::Set
        | ListAccessCapability::Replace
        | ListAccessCapability::Reverse
        | ListAccessCapability::Sublist => false,
    }
}

impl<S> ListAccess<S> for DynamicListVariableSlot<S>
where
    S: Clone + Send + Sync + 'static,
{
    type Element = usize;

    fn entity_type_name(&self) -> &'static str {
        self.entity_type_name
    }

    fn variable_name(&self) -> &'static str {
        self.variable_name
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index()
    }

    fn entity_count(&self, solution: &S) -> usize {
        self.entity_count(solution)
    }

    fn element_count(&self, solution: &S) -> usize {
        self.element_count(solution)
    }

    fn index_to_element(&self, solution: &S, element_index: usize) -> Option<Self::Element> {
        self.element(solution, element_index)
    }

    fn element_source_key(&self, _solution: &S, element: &Self::Element) -> usize {
        *element
    }

    fn assigned_elements(&self, solution: &S) -> Vec<Self::Element> {
        self.assigned_elements(solution)
    }

    fn list_len(&self, solution: &S, entity: usize) -> usize {
        self.list_len(solution, entity)
    }

    fn list_get(&self, solution: &S, entity: usize, pos: usize) -> Option<Self::Element> {
        self.list_get(solution, entity, pos)
    }

    fn list_insert(&self, solution: &mut S, entity: usize, pos: usize, value: Self::Element) {
        self.list_insert(solution, entity, pos, value);
    }

    fn list_remove(&self, solution: &mut S, entity: usize, pos: usize) -> Option<Self::Element> {
        self.list_remove(solution, entity, pos)
    }

    fn list_set(
        &self,
        solution: &mut S,
        entity: usize,
        pos: usize,
        value: Self::Element,
    ) -> Result<(), ListAccessError> {
        if !dynamic_access_has(self.access_capabilities(), ListAccessCapability::Set)
            || !self.list_set(solution, entity, pos, value)
        {
            return Err(dynamic_error(self, ListAccessCapability::Set));
        }
        Ok(())
    }

    fn list_reverse(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<(), ListAccessError> {
        if !dynamic_access_has(self.access_capabilities(), ListAccessCapability::Reverse)
            || !self.list_reverse(solution, entity, start, end)
        {
            return Err(dynamic_error(self, ListAccessCapability::Reverse));
        }
        Ok(())
    }

    fn sublist_remove(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<Vec<Self::Element>, ListAccessError> {
        if !dynamic_access_has(self.access_capabilities(), ListAccessCapability::Sublist) {
            return Err(dynamic_error(self, ListAccessCapability::Sublist));
        }
        self.sublist_remove(solution, entity, start, end)
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::Sublist))
    }

    fn sublist_insert(
        &self,
        solution: &mut S,
        entity: usize,
        pos: usize,
        values: Vec<Self::Element>,
    ) -> Result<(), ListAccessError> {
        if !dynamic_access_has(self.access_capabilities(), ListAccessCapability::Sublist)
            || !self.sublist_insert(solution, entity, pos, values)
        {
            return Err(dynamic_error(self, ListAccessCapability::Sublist));
        }
        Ok(())
    }

    fn element_owner(
        &self,
        solution: &S,
        element: &Self::Element,
    ) -> Result<Option<usize>, ListAccessError> {
        let metadata = self
            .metadata()
            .filter(|metadata| {
                dynamic_metadata_has(metadata.capabilities(), ListAccessCapability::ElementOwner)
            })
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::ElementOwner))?;
        Ok(metadata.element_owner(solution, *element))
    }

    fn construction_order_key(
        &self,
        solution: &S,
        element: Self::Element,
    ) -> Result<i64, ListAccessError> {
        let metadata = self
            .metadata()
            .filter(|metadata| {
                dynamic_metadata_has(
                    metadata.capabilities(),
                    ListAccessCapability::ConstructionOrderKey,
                )
            })
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::ConstructionOrderKey))?;
        metadata
            .construction_order_key(solution, element)
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::ConstructionOrderKey))
    }

    fn extend_precedence_successors(
        &self,
        solution: &S,
        element: Self::Element,
        successors: &mut Vec<Self::Element>,
    ) -> Result<(), ListAccessError> {
        let metadata = self
            .metadata()
            .filter(|metadata| metadata.capabilities().precedence_successors)
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::Precedence))?;
        metadata
            .extend_precedence_successors(solution, element, successors)
            .then_some(())
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::Precedence))
    }

    fn precedence_duration(
        &self,
        solution: &S,
        element: Self::Element,
    ) -> Result<usize, ListAccessError> {
        let metadata = self
            .metadata()
            .filter(|metadata| metadata.capabilities().precedence_duration)
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::Precedence))?;
        metadata
            .precedence_duration(solution, element)
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::Precedence))
    }

    fn cross_position_distance(
        &self,
        solution: &S,
        from_entity: usize,
        from_position: usize,
        to_entity: usize,
        to_position: usize,
    ) -> Result<f64, ListAccessError> {
        let metadata = self
            .metadata()
            .filter(|metadata| {
                dynamic_metadata_has(
                    metadata.capabilities(),
                    ListAccessCapability::CrossPositionDistance,
                )
            })
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::CrossPositionDistance))?;
        metadata
            .cross_position_distance(solution, from_entity, from_position, to_entity, to_position)
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::CrossPositionDistance))
    }

    fn intra_position_distance(
        &self,
        solution: &S,
        entity: usize,
        from_position: usize,
        to_position: usize,
    ) -> Result<f64, ListAccessError> {
        let metadata = self
            .metadata()
            .filter(|metadata| {
                dynamic_metadata_has(
                    metadata.capabilities(),
                    ListAccessCapability::IntraPositionDistance,
                )
            })
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::IntraPositionDistance))?;
        metadata
            .intra_position_distance(solution, entity, from_position, to_position)
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::IntraPositionDistance))
    }
}
