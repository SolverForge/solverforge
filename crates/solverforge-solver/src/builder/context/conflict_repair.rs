use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConflictRepairEdit {
    pub descriptor_index: usize,
    pub entity_index: usize,
    pub variable_name: &'static str,
    pub to_value: Option<usize>,
}

impl ConflictRepairEdit {
    pub fn set_scalar(
        descriptor_index: usize,
        entity_index: usize,
        variable_name: &'static str,
        to_value: Option<usize>,
    ) -> Self {
        Self {
            descriptor_index,
            entity_index,
            variable_name,
            to_value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConflictRepairSpec {
    pub reason: &'static str,
    pub edits: Vec<ConflictRepairEdit>,
}

impl ConflictRepairSpec {
    pub fn new(reason: &'static str, edits: Vec<ConflictRepairEdit>) -> Self {
        Self { reason, edits }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConflictRepairLimits {
    pub max_matches_per_step: usize,
    pub max_repairs_per_match: usize,
    pub max_moves_per_step: usize,
}

pub type ConflictRepairProvider<S> = fn(&S, ConflictRepairLimits) -> Vec<ConflictRepairSpec>;

pub struct ConflictRepairProviderEntry<S> {
    pub constraint_name: &'static str,
    pub provider: ConflictRepairProvider<S>,
}

impl<S> ConflictRepairProviderEntry<S> {
    pub const fn new(constraint_name: &'static str, provider: ConflictRepairProvider<S>) -> Self {
        Self {
            constraint_name,
            provider,
        }
    }
}

impl<S> fmt::Debug for ConflictRepairProviderEntry<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConflictRepairProviderEntry")
            .field("constraint_name", &self.constraint_name)
            .finish_non_exhaustive()
    }
}

impl<S> Clone for ConflictRepairProviderEntry<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ConflictRepairProviderEntry<S> {}
