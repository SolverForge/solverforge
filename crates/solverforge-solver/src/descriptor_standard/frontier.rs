use crate::phase::construction::ConstructionSlotId;

#[derive(Debug, Default)]
pub(crate) struct StandardConstructionFrontier {
    completed_at_revision: Vec<Vec<u64>>,
}

impl StandardConstructionFrontier {
    pub(crate) fn new(binding_count: usize) -> Self {
        Self {
            completed_at_revision: vec![Vec::new(); binding_count],
        }
    }

    pub(crate) fn reset(&mut self) {
        for slots in &mut self.completed_at_revision {
            slots.clear();
        }
    }

    pub(crate) fn is_completed(&self, slot_id: ConstructionSlotId, solution_revision: u64) -> bool {
        self.completed_at_revision
            .get(slot_id.binding_index())
            .and_then(|slots| slots.get(slot_id.entity_index()))
            .is_some_and(|completed_revision| *completed_revision == solution_revision)
    }

    pub(crate) fn mark_completed(&mut self, slot_id: ConstructionSlotId, solution_revision: u64) {
        let slot = self.ensure_slot(slot_id);
        *slot = solution_revision;
    }

    fn ensure_slot(&mut self, slot_id: ConstructionSlotId) -> &mut u64 {
        if self.completed_at_revision.len() <= slot_id.binding_index() {
            self.completed_at_revision
                .resize_with(slot_id.binding_index() + 1, Vec::new);
        }
        let slots = &mut self.completed_at_revision[slot_id.binding_index()];
        if slots.len() <= slot_id.entity_index() {
            slots.resize(slot_id.entity_index() + 1, 0);
        }
        &mut slots[slot_id.entity_index()]
    }
}
