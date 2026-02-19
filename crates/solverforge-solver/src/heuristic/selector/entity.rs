//! Entity selectors for iterating over planning entities

use std::any::Any;
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
    ) -> impl Iterator<Item = EntityReference> + 'a;

    /// Returns the approximate number of entities.
    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize;

    /// Returns true if this selector may return the same entity multiple times.
    fn is_never_ending(&self) -> bool {
        false
    }
}

/// An entity selector that iterates over all entities from the solution.
#[derive(Clone)]
pub struct FromSolutionEntitySelector {
    /// The descriptor index to select from.
    descriptor_index: usize,
    /// Whether to skip pinned entities.
    skip_pinned: bool,
    /// Optional function to test whether an entity (as `&dyn Any`) is pinned.
    ///
    /// Required when `skip_pinned` is true. The function receives the entity
    /// returned by `ScoreDirector::get_entity` and returns `true` if pinned.
    is_pinned_fn: Option<fn(&dyn Any) -> bool>,
}

impl Debug for FromSolutionEntitySelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FromSolutionEntitySelector")
            .field("descriptor_index", &self.descriptor_index)
            .field("skip_pinned", &self.skip_pinned)
            .field("has_is_pinned_fn", &self.is_pinned_fn.is_some())
            .finish()
    }
}

impl FromSolutionEntitySelector {
    /// Creates a new entity selector for the given descriptor index.
    pub fn new(descriptor_index: usize) -> Self {
        Self {
            descriptor_index,
            skip_pinned: false,
            is_pinned_fn: None,
        }
    }

    /// Creates an entity selector that skips pinned entities.
    ///
    /// When `skip` is `true`, you must also call
    /// [`with_is_pinned_fn`](Self::with_is_pinned_fn) to supply a predicate,
    /// otherwise no filtering occurs.
    pub fn with_skip_pinned(mut self, skip: bool) -> Self {
        self.skip_pinned = skip;
        self
    }

    /// Sets the function used to determine whether an entity is pinned.
    ///
    /// The function receives the entity as `&dyn Any` (the value returned by
    /// `ScoreDirector::get_entity`) and returns `true` if the entity is pinned.
    ///
    /// Only consulted when `skip_pinned` is `true`.
    pub fn with_is_pinned_fn(mut self, f: fn(&dyn Any) -> bool) -> Self {
        self.is_pinned_fn = Some(f);
        self
    }
}

impl<S: PlanningSolution> EntitySelector<S> for FromSolutionEntitySelector {
    fn iter<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = EntityReference> + 'a {
        let count = score_director
            .entity_count(self.descriptor_index)
            .unwrap_or(0);

        let desc_idx = self.descriptor_index;
        let skip_pinned = self.skip_pinned;
        let is_pinned_fn = self.is_pinned_fn;

        (0..count)
            .filter(move |&i| {
                if skip_pinned {
                    if let Some(pinned_fn) = is_pinned_fn {
                        if let Some(entity) = score_director.get_entity(desc_idx, i) {
                            if pinned_fn(entity) {
                                return false;
                            }
                        }
                    }
                }
                true
            })
            .map(move |i| EntityReference::new(desc_idx, i))
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
            assert_eq!(queen.column, i as i64);
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
