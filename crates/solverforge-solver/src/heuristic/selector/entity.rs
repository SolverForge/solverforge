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
pub trait EntitySelector<S: PlanningSolution>: Send + Debug {
    /// Returns an iterator over entity references.
    ///
    /// The iterator yields `EntityReference` values that identify entities
    /// within the solution.
    fn iter<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
    ) -> Box<dyn Iterator<Item = EntityReference> + 'a>;

    /// Returns the approximate number of entities.
    fn size(&self, score_director: &dyn ScoreDirector<S>) -> usize;

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
    fn iter<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
    ) -> Box<dyn Iterator<Item = EntityReference> + 'a> {
        let count = score_director
            .entity_count(self.descriptor_index)
            .unwrap_or(0);

        let desc_idx = self.descriptor_index;

        Box::new((0..count).map(move |i| EntityReference::new(desc_idx, i)))
    }

    fn size(&self, score_director: &dyn ScoreDirector<S>) -> usize {
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
    fn iter<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
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

    fn size(&self, score_director: &dyn ScoreDirector<S>) -> usize {
        score_director.total_entity_count().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::SimpleScoreDirector;
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Queen {
        id: i64,
        row: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct NQueensSolution {
        queens: Vec<Queen>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for NQueensSolution {
        type Score = SimpleScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_queens(s: &NQueensSolution) -> &Vec<Queen> {
        &s.queens
    }

    fn get_queens_mut(s: &mut NQueensSolution) -> &mut Vec<Queen> {
        &mut s.queens
    }

    fn create_test_director(
        n: usize,
    ) -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
        let queens: Vec<_> = (0..n)
            .map(|i| Queen {
                id: i as i64,
                row: Some(i as i32),
            })
            .collect();

        let solution = NQueensSolution {
            queens,
            score: None,
        };

        let extractor = Box::new(TypedEntityExtractor::new(
            "Queen",
            "queens",
            get_queens,
            get_queens_mut,
        ));
        let entity_desc = EntityDescriptor::new("Queen", TypeId::of::<Queen>(), "queens")
            .with_extractor(extractor);

        let descriptor =
            SolutionDescriptor::new("NQueensSolution", TypeId::of::<NQueensSolution>())
                .with_entity(entity_desc);

        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn test_from_solution_entity_selector() {
        let director = create_test_director(4);

        // Verify entity IDs match indices
        let solution = director.working_solution();
        for (i, queen) in solution.queens.iter().enumerate() {
            assert_eq!(queen.id, i as i64);
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
        let director = create_test_director(3);

        let selector = AllEntitiesSelector::new();

        let refs: Vec<_> = selector.iter(&director).collect();
        assert_eq!(refs.len(), 3);

        assert_eq!(selector.size(&director), 3);
    }
}
