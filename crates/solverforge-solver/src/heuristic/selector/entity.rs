//! Entity selectors for iterating over planning entities

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

/// A reference to an entity within a solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityReference {
    /// Index of the entity descriptor.
    pub descriptor_index: usize,
    /// Index of the entity within its collection.
    pub entity_index: usize,
}

impl EntityReference {
    /// Creates a new entity reference.
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
    /// Returns an iterator over entity references.
    ///
    /// The iterator yields `EntityReference` values that identify entities
    /// within the solution.
    fn iter<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = EntityReference> + 'a>;

    /// Returns the approximate number of entities.
    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize;

    /// Returns true if this selector may return the same entity multiple times.
    fn is_never_ending(&self) -> bool {
        false
    }
}

/// An entity selector that iterates over all entities from the solution.
#[derive(Debug, Clone)]
pub struct FromSolutionEntitySelector {
    /// The descriptor index to select from.
    descriptor_index: usize,
    /// Whether to skip pinned entities.
    skip_pinned: bool,
}

impl FromSolutionEntitySelector {
    /// Creates a new entity selector for the given descriptor index.
    pub fn new(descriptor_index: usize) -> Self {
        Self {
            descriptor_index,
            skip_pinned: false,
        }
    }

    /// Creates an entity selector that skips pinned entities.
    pub fn with_skip_pinned(mut self, skip: bool) -> Self {
        self.skip_pinned = skip;
        self
    }
}

impl<S: PlanningSolution> EntitySelector<S> for FromSolutionEntitySelector {
    fn iter<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = EntityReference> + 'a> {
        let count = score_director
            .entity_count(self.descriptor_index)
            .unwrap_or(0);

        let desc_idx = self.descriptor_index;

        Box::new((0..count).map(move |i| EntityReference::new(desc_idx, i)))
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        score_director
            .entity_count(self.descriptor_index)
            .unwrap_or(0)
    }
}

/// An entity selector that iterates over all entities from all descriptors.
#[derive(Debug, Clone, Default)]
pub struct AllEntitiesSelector;

impl AllEntitiesSelector {
    /// Creates a new selector for all entities.
    pub fn new() -> Self {
        Self
    }
}

impl<S: PlanningSolution> EntitySelector<S> for AllEntitiesSelector {
    fn iter<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = EntityReference> + 'a> {
        let desc = score_director.solution_descriptor();
        let descriptor_count = desc.entity_descriptors.len();

        let mut refs = Vec::new();
        for desc_idx in 0..descriptor_count {
            let count = score_director.entity_count(desc_idx).unwrap_or(0);
            for entity_idx in 0..count {
                refs.push(EntityReference::new(desc_idx, entity_idx));
            }
        }

        Box::new(refs.into_iter())
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        score_director.total_entity_count().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_simple_nqueens_director;

    #[test]
    fn test_from_solution_entity_selector() {
        let director = create_simple_nqueens_director(4);

        // Verify column values match indices (column is set to index in create_uninitialized_nqueens)
        let solution = director.working_solution();
        for (i, queen) in solution.queens.iter().enumerate() {
            assert_eq!(queen.column, i as i32);
        }

        let selector = FromSolutionEntitySelector::new(0);

        let refs: Vec<_> = selector.iter(&director).collect();
        assert_eq!(refs.len(), 4);
        assert_eq!(refs[0], EntityReference::new(0, 0));
        assert_eq!(refs[1], EntityReference::new(0, 1));
        assert_eq!(refs[2], EntityReference::new(0, 2));
        assert_eq!(refs[3], EntityReference::new(0, 3));

        assert_eq!(selector.size(&director), 4);
    }

    #[test]
    fn test_all_entities_selector() {
        let director = create_simple_nqueens_director(3);

        let selector = AllEntitiesSelector::new();

        let refs: Vec<_> = selector.iter(&director).collect();
        assert_eq!(refs.len(), 3);

        assert_eq!(selector.size(&director), 3);
    }
}
