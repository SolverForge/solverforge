// Entity selectors for iterating over planning entities

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

// A reference to an entity within a solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityReference {
    // Index of the entity descriptor.
    pub descriptor_index: usize,
    // Index of the entity within its collection.
    pub entity_index: usize,
}

impl EntityReference {
    pub fn new(descriptor_index: usize, entity_index: usize) -> Self {
        Self {
            descriptor_index,
            entity_index,
        }
    }
}

/// Trait for selecting entities from a planning solution.
///
/// Entity selectors provide an iteration order over the entities that
/// the solver will consider for moves.
///
/// # Type Parameters
/// * `S` - The planning solution type
pub trait EntitySelector<S: PlanningSolution>: Send + Debug {
    /* Returns an iterator over entity references.

    The iterator yields `EntityReference` values that identify entities
    within the solution.
    */
    fn iter<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = EntityReference> + 'a;

    fn size<D: Director<S>>(&self, score_director: &D) -> usize;

    // Returns true if this selector may return the same entity multiple times.
    fn is_never_ending(&self) -> bool {
        false
    }
}

// An entity selector that iterates over all entities from the solution.
#[derive(Clone, Debug)]
pub struct FromSolutionEntitySelector {
    // The descriptor index to select from.
    descriptor_index: usize,
}

impl FromSolutionEntitySelector {
    pub fn new(descriptor_index: usize) -> Self {
        Self { descriptor_index }
    }
}

impl<S: PlanningSolution> EntitySelector<S> for FromSolutionEntitySelector {
    fn iter<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = EntityReference> + 'a {
        let count = score_director
            .entity_count(self.descriptor_index)
            .unwrap_or(0);
        let desc_idx = self.descriptor_index;
        (0..count).map(move |i| EntityReference::new(desc_idx, i))
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        score_director
            .entity_count(self.descriptor_index)
            .unwrap_or(0)
    }
}

// An entity selector that iterates over all entities from all descriptors.
#[derive(Debug, Clone, Default)]
pub struct AllEntitiesSelector;

impl AllEntitiesSelector {
    pub fn new() -> Self {
        Self
    }
}

impl<S: PlanningSolution> EntitySelector<S> for AllEntitiesSelector {
    fn iter<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = EntityReference> + 'a {
        let desc = score_director.solution_descriptor();
        let descriptor_count = desc.entity_descriptors.len();

        let mut refs = Vec::new();
        for desc_idx in 0..descriptor_count {
            let count = score_director.entity_count(desc_idx).unwrap_or(0);
            for entity_idx in 0..count {
                refs.push(EntityReference::new(desc_idx, entity_idx));
            }
        }

        refs.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        score_director.total_entity_count().unwrap_or(0)
    }
}

#[cfg(test)]
#[path = "entity_tests.rs"]
mod tests;
