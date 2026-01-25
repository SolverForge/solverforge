//! Entity selectors for iterating over planning entities

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
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
pub trait EntitySelector<S>: Send + Debug
where
    S: PlanningSolution,
    S::Score: Score,
{
    /// Returns an iterator over entity references.
    ///
    /// The iterator yields `EntityReference` values that identify entities
    /// within the solution.
    fn iter<'a, C>(
        &'a self,
        score_director: &'a ScoreDirector<S, C>,
    ) -> Box<dyn Iterator<Item = EntityReference> + 'a>
    where
        C: ConstraintSet<S, S::Score>;

    /// Returns the approximate number of entities.
    fn size<C>(&self, score_director: &ScoreDirector<S, C>) -> usize
    where
        C: ConstraintSet<S, S::Score>;

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

impl<S> EntitySelector<S> for FromSolutionEntitySelector
where
    S: PlanningSolution,
    S::Score: Score,
{
    fn iter<'a, C>(
        &'a self,
        score_director: &'a ScoreDirector<S, C>,
    ) -> Box<dyn Iterator<Item = EntityReference> + 'a>
    where
        C: ConstraintSet<S, S::Score>,
    {
        let count = score_director.entity_count(self.descriptor_index);

        let desc_idx = self.descriptor_index;

        Box::new((0..count).map(move |i| EntityReference::new(desc_idx, i)))
    }

    fn size<C>(&self, score_director: &ScoreDirector<S, C>) -> usize
    where
        C: ConstraintSet<S, S::Score>,
    {
        score_director.entity_count(self.descriptor_index)
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

impl<S> EntitySelector<S> for AllEntitiesSelector
where
    S: PlanningSolution,
    S::Score: Score,
{
    fn iter<'a, C>(
        &'a self,
        score_director: &'a ScoreDirector<S, C>,
    ) -> Box<dyn Iterator<Item = EntityReference> + 'a>
    where
        C: ConstraintSet<S, S::Score>,
    {
        let desc = score_director.solution_descriptor();
        let descriptor_count = desc.entity_descriptors.len();

        // Lazy iteration - no Vec allocation
        Box::new((0..descriptor_count).flat_map(move |desc_idx| {
            let count = score_director.entity_count(desc_idx);
            (0..count).map(move |entity_idx| EntityReference::new(desc_idx, entity_idx))
        }))
    }

    fn size<C>(&self, score_director: &ScoreDirector<S, C>) -> usize
    where
        C: ConstraintSet<S, S::Score>,
    {
        score_director.total_entity_count()
    }
}
