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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ConstructionListElementId {
    list_index: usize,
    element_index: usize,
}

impl ConstructionListElementId {
    pub(crate) fn new(list_index: usize, element_index: usize) -> Self {
        Self {
            list_index,
            element_index,
        }
    }

    pub(crate) fn list_index(self) -> usize {
        self.list_index
    }

    pub(crate) fn element_index(self) -> usize {
        self.element_index
    }
}
