use super::*;

#[derive(Debug)]
struct DynamicAccess;

impl DynamicListAccess<ListPlan> for DynamicAccess {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn entity_count(&self, plan: &ListPlan) -> usize {
        entity_count(plan)
    }

    fn element_count(&self, plan: &ListPlan) -> usize {
        element_count(plan)
    }

    fn element(&self, plan: &ListPlan, index: usize) -> Option<usize> {
        plan.elements.get(index).copied()
    }

    fn assigned_elements(&self, plan: &ListPlan) -> Vec<usize> {
        assigned_elements(plan)
    }

    fn len(&self, plan: &ListPlan, entity: usize) -> usize {
        list_len(plan, entity)
    }

    fn get(&self, plan: &ListPlan, entity: usize, position: usize) -> Option<usize> {
        list_get(plan, entity, position)
    }

    fn insert(&self, plan: &mut ListPlan, entity: usize, position: usize, value: usize) {
        list_insert(plan, entity, position, value);
    }

    fn remove(&self, plan: &mut ListPlan, entity: usize, position: usize) -> Option<usize> {
        list_remove(plan, entity, position)
    }

    fn capabilities(&self) -> DynamicListAccessCapabilities {
        DynamicListAccessCapabilities {
            set: true,
            reverse: true,
            sublist: true,
            ..DynamicListAccessCapabilities::default()
        }
    }

    fn set(&self, plan: &mut ListPlan, entity: usize, position: usize, value: usize) -> bool {
        list_set(plan, entity, position, value);
        true
    }

    fn reverse(&self, plan: &mut ListPlan, entity: usize, start: usize, end: usize) -> bool {
        list_reverse(plan, entity, start, end);
        true
    }

    fn sublist_remove(
        &self,
        plan: &mut ListPlan,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Option<Vec<usize>> {
        Some(sublist_remove(plan, entity, start, end))
    }

    fn sublist_insert(
        &self,
        plan: &mut ListPlan,
        entity: usize,
        position: usize,
        values: Vec<usize>,
    ) -> bool {
        sublist_insert(plan, entity, position, values);
        true
    }
}

#[derive(Debug)]
struct DynamicMetadata {
    precedence: bool,
}

impl DynamicListMetadata<ListPlan> for DynamicMetadata {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn capabilities(&self) -> DynamicListMetadataCapabilities {
        DynamicListMetadataCapabilities {
            precedence_duration: self.precedence,
            precedence_successors: self.precedence,
            cross_position_distance: true,
            intra_position_distance: true,
            ..DynamicListMetadataCapabilities::default()
        }
    }

    fn element_owner(&self, _: &ListPlan, _: usize) -> Option<usize> {
        None
    }

    fn construction_order_key(&self, _: &ListPlan, _: usize) -> Option<i64> {
        None
    }

    fn precedence_duration(&self, _: &ListPlan, _: usize) -> Option<usize> {
        self.precedence.then_some(1)
    }

    fn extend_precedence_successors(
        &self,
        _: &ListPlan,
        element: usize,
        successors: &mut Vec<usize>,
    ) -> bool {
        if !self.precedence {
            return false;
        }
        if element % 4 != 3 {
            successors.push(element + 1);
        }
        true
    }

    fn cross_position_distance(
        &self,
        _: &ListPlan,
        from_entity: usize,
        from_position: usize,
        to_entity: usize,
        to_position: usize,
    ) -> Option<f64> {
        Some(distance(from_entity, from_position, to_entity, to_position) as f64)
    }

    fn intra_position_distance(
        &self,
        _: &ListPlan,
        entity: usize,
        from_position: usize,
        to_position: usize,
    ) -> Option<f64> {
        Some(distance(entity, from_position, entity, to_position) as f64)
    }

    fn route_depot(&self, _: &ListPlan, _: usize) -> Option<usize> {
        None
    }

    fn route_distance(&self, _: &ListPlan, _: usize, _: usize, _: usize) -> Option<i64> {
        None
    }

    fn route_feasible(&self, _: &ListPlan, _: usize, _: &[usize]) -> Option<bool> {
        None
    }

    fn savings_depot(&self, _: &ListPlan, _: usize) -> Option<usize> {
        None
    }

    fn savings_metric_class(&self, _: &ListPlan, _: usize) -> Option<usize> {
        None
    }

    fn savings_distance(&self, _: &ListPlan, _: usize, _: usize, _: usize) -> Option<i64> {
        None
    }

    fn savings_feasible(&self, _: &ListPlan, _: usize, _: &[usize]) -> Option<bool> {
        None
    }
}

fn dynamic_variable_slot_with_precedence(precedence: bool) -> DynamicListVariableSlot<ListPlan> {
    DynamicListVariableSlot::with_access_and_metadata(
        EntityClassId(0),
        VariableId(0),
        "Vehicle",
        "visits",
        Arc::new(DynamicAccess),
        Arc::new(DynamicMetadata { precedence }),
    )
    .expect("dynamic test access and metadata have matching identities")
    .resolved_against(&descriptor())
    .expect("dynamic test slot resolves against the list descriptor")
}

pub(crate) fn dynamic_variable_slot() -> DynamicListVariableSlot<ListPlan> {
    dynamic_variable_slot_with_precedence(false)
}

pub(crate) fn dynamic_slot() -> Slot {
    Slot::from_dynamic(dynamic_variable_slot())
}

pub(crate) fn dynamic_slot_for(kind: ListLeafKind) -> Slot {
    Slot::from_dynamic(dynamic_variable_slot_with_precedence(
        kind == ListLeafKind::Precedence,
    ))
}
