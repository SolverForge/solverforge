//! Entity selectors for iterating over planning entities

use std::fmt::Debug;

#[cfg(test)]
use solverforge_core::domain::PlanningSolution;
#[cfg(test)]
use solverforge_scoring::Director;

/// A reference to an entity within a solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct EntityReference {
    /// Index of the entity descriptor.
    pub descriptor_index: usize,
    /// Index of the entity within its collection.
    pub entity_index: usize,
}

impl EntityReference {
    pub(crate) fn new(descriptor_index: usize, entity_index: usize) -> Self {
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
#[cfg(test)]
pub(crate) trait EntitySelector<S: PlanningSolution>: Send + Debug {
    /// Returns an iterator over entity references.
    ///
    /// The iterator yields `EntityReference` values that identify entities
    /// within the solution.
    fn iter<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = EntityReference> + 'a;

    fn size<D: Director<S>>(&self, score_director: &D) -> usize;
}

/// An entity selector that iterates over all entities from the solution.
#[derive(Clone, Debug)]
#[cfg(test)]
pub(crate) struct FromSolutionEntitySelector {
    // The descriptor index to select from.
    descriptor_index: usize,
}

#[cfg(test)]
impl FromSolutionEntitySelector {
    pub(crate) fn new(descriptor_index: usize) -> Self {
        Self { descriptor_index }
    }
}

#[cfg(test)]
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

#[cfg(test)]
#[path = "entity_tests.rs"]
mod tests;
