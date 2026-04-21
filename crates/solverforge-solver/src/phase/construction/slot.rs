#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ConstructionSlotId {
    binding_index: usize,
    entity_index: usize,
}

impl ConstructionSlotId {
    pub(crate) fn new(binding_index: usize, entity_index: usize) -> Self {
        Self {
            binding_index,
            entity_index,
        }
    }

    pub(crate) fn binding_index(self) -> usize {
        self.binding_index
    }

    pub(crate) fn entity_index(self) -> usize {
        self.entity_index
    }
}
