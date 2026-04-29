use std::collections::HashMap;

use super::{
    ConstructionGroupSlotId, ConstructionGroupSlotKey, ConstructionListElementId,
    ConstructionSlotId,
};

#[derive(Debug, Default, Clone)]
pub(crate) struct ConstructionFrontier {
    scalar_completed_at_revision: Vec<Vec<u64>>,
    group_completed_at_revision: Vec<HashMap<ConstructionGroupSlotKey, u64>>,
    list_completed_at_revision: Vec<Vec<u64>>,
}

impl ConstructionFrontier {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn reset(&mut self) {
        for slots in &mut self.scalar_completed_at_revision {
            slots.clear();
        }
        for groups in &mut self.group_completed_at_revision {
            groups.clear();
        }
        for elements in &mut self.list_completed_at_revision {
            elements.clear();
        }
    }

    pub(crate) fn is_scalar_completed(
        &self,
        slot_id: ConstructionSlotId,
        solution_revision: u64,
    ) -> bool {
        self.scalar_completed_at_revision
            .get(slot_id.binding_index())
            .and_then(|slots| slots.get(slot_id.entity_index()))
            .is_some_and(|completed_revision| *completed_revision == solution_revision)
    }

    pub(crate) fn mark_scalar_completed(
        &mut self,
        slot_id: ConstructionSlotId,
        solution_revision: u64,
    ) {
        let slot = self.ensure_scalar_slot(slot_id);
        *slot = solution_revision;
    }

    pub(crate) fn is_group_completed(
        &self,
        slot_id: ConstructionGroupSlotId,
        solution_revision: u64,
    ) -> bool {
        self.group_completed_at_revision
            .get(slot_id.group_index())
            .and_then(|groups| groups.get(slot_id.key()))
            .is_some_and(|completed_revision| *completed_revision == solution_revision)
    }

    pub(crate) fn mark_group_completed(
        &mut self,
        slot_id: ConstructionGroupSlotId,
        solution_revision: u64,
    ) {
        let slot = self.ensure_group_slot(&slot_id);
        *slot = solution_revision;
    }

    pub(crate) fn is_list_completed(
        &self,
        element_id: ConstructionListElementId,
        solution_revision: u64,
    ) -> bool {
        self.list_completed_at_revision
            .get(element_id.list_index())
            .and_then(|elements| elements.get(element_id.element_index()))
            .is_some_and(|completed_revision| *completed_revision == solution_revision)
    }

    pub(crate) fn mark_list_completed(
        &mut self,
        element_id: ConstructionListElementId,
        solution_revision: u64,
    ) {
        let element = self.ensure_list_element(element_id);
        *element = solution_revision;
    }

    fn ensure_scalar_slot(&mut self, slot_id: ConstructionSlotId) -> &mut u64 {
        if self.scalar_completed_at_revision.len() <= slot_id.binding_index() {
            self.scalar_completed_at_revision
                .resize_with(slot_id.binding_index() + 1, Vec::new);
        }
        let slots = &mut self.scalar_completed_at_revision[slot_id.binding_index()];
        if slots.len() <= slot_id.entity_index() {
            slots.resize(slot_id.entity_index() + 1, 0);
        }
        &mut slots[slot_id.entity_index()]
    }

    fn ensure_group_slot(&mut self, slot_id: &ConstructionGroupSlotId) -> &mut u64 {
        if self.group_completed_at_revision.len() <= slot_id.group_index() {
            self.group_completed_at_revision
                .resize_with(slot_id.group_index() + 1, HashMap::new);
        }
        let slots = &mut self.group_completed_at_revision[slot_id.group_index()];
        slots.entry(slot_id.key().clone()).or_default()
    }

    fn ensure_list_element(&mut self, element_id: ConstructionListElementId) -> &mut u64 {
        if self.list_completed_at_revision.len() <= element_id.list_index() {
            self.list_completed_at_revision
                .resize_with(element_id.list_index() + 1, Vec::new);
        }
        let elements = &mut self.list_completed_at_revision[element_id.list_index()];
        if elements.len() <= element_id.element_index() {
            elements.resize(element_id.element_index() + 1, 0);
        }
        &mut elements[element_id.element_index()]
    }
}
