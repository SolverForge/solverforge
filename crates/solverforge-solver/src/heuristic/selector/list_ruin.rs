//! List ruin move selector for Large Neighborhood Search on list variables.
//!
//! Generates `ListRuinMove` instances that remove elements from list variables,
//! enabling exploration of distant regions in the solution space.
//!
//! # Zero-Erasure Design
//!
//! No value type parameter. Uses VariableOperations trait for list access.

use std::fmt::Debug;
use std::marker::PhantomData;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::list_ruin::ListRuinMove;
use crate::operations::VariableOperations;

use super::MoveSelector;

/// A move selector that generates `ListRuinMove` instances for Large Neighborhood Search.
///
/// Selects random subsets of elements from list variables to "ruin" (remove),
/// enabling a construction heuristic to reinsert them in better positions.
///
/// # Type Parameters
/// * `S` - The planning solution type (must implement VariableOperations)
///
/// # Zero-Erasure
///
/// Uses VariableOperations trait for list access. No function pointers required.
pub struct ListRuinMoveSelector<S> {
    /// Minimum elements to remove per move.
    min_ruin_count: usize,
    /// Maximum elements to remove per move.
    max_ruin_count: usize,
    /// Random seed for reproducible subset selection.
    seed: Option<u64>,
    /// Variable name.
    variable_name: &'static str,
    /// Entity descriptor index.
    descriptor_index: usize,
    /// Number of ruin moves to generate per iteration.
    moves_per_step: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for ListRuinMoveSelector<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListRuinMoveSelector")
            .field("min_ruin_count", &self.min_ruin_count)
            .field("max_ruin_count", &self.max_ruin_count)
            .field("moves_per_step", &self.moves_per_step)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S> ListRuinMoveSelector<S> {
    /// Creates a new list ruin move selector.
    ///
    /// # Arguments
    /// * `min_ruin_count` - Minimum elements to remove (at least 1)
    /// * `max_ruin_count` - Maximum elements to remove
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    ///
    /// # Panics
    /// Panics if `min_ruin_count` is 0 or `max_ruin_count < min_ruin_count`.
    pub fn new(
        min_ruin_count: usize,
        max_ruin_count: usize,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        assert!(min_ruin_count >= 1, "min_ruin_count must be at least 1");
        assert!(
            max_ruin_count >= min_ruin_count,
            "max_ruin_count must be >= min_ruin_count"
        );

        Self {
            min_ruin_count,
            max_ruin_count,
            seed: None,
            variable_name,
            descriptor_index,
            moves_per_step: 10,
            _phantom: PhantomData,
        }
    }

    /// Sets the number of ruin moves to generate per iteration.
    ///
    /// Default is 10.
    pub fn with_moves_per_step(mut self, count: usize) -> Self {
        self.moves_per_step = count;
        self
    }

    /// Sets the random seed for reproducible subset selection.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Creates a random number generator.
    fn create_rng(&self) -> StdRng {
        match self.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_os_rng(),
        }
    }
}

impl<S> MoveSelector<S, ListRuinMove<S>> for ListRuinMoveSelector<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = ListRuinMove<S>> + 'a> {
        let solution = score_director.working_solution();
        let total_entities = solution.entity_count();
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;
        let min_ruin = self.min_ruin_count;
        let max_ruin = self.max_ruin_count;
        let moves_count = self.moves_per_step;

        if total_entities == 0 {
            return Box::new(std::iter::empty());
        }

        // Pre-generate moves using RNG
        let mut rng = self.create_rng();
        let moves: Vec<ListRuinMove<S>> = (0..moves_count)
            .filter_map(|_| {
                // Pick a random entity
                let entity_idx = rng.random_range(0..total_entities);
                let list_length = solution.list_len(entity_idx);

                if list_length == 0 {
                    return None;
                }

                // Clamp ruin count to available elements
                let min = min_ruin.min(list_length);
                let max = max_ruin.min(list_length);
                let ruin_count = if min == max {
                    min
                } else {
                    rng.random_range(min..=max)
                };

                // Fisher-Yates partial shuffle to select random indices
                let mut indices: SmallVec<[usize; 8]> = (0..list_length).collect();
                for i in 0..ruin_count {
                    let j = rng.random_range(i..list_length);
                    indices.swap(i, j);
                }
                indices.truncate(ruin_count);

                Some(ListRuinMove::new(
                    entity_idx,
                    &indices,
                    variable_name,
                    descriptor_index,
                ))
            })
            .collect();

        Box::new(moves.into_iter())
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        let total = score_director.working_solution().entity_count();
        if total == 0 {
            return 0;
        }
        self.moves_per_step
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::domain::SolutionDescriptor;
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::SimpleScoreDirector;
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Route {
        stops: Vec<usize>,
    }

    #[derive(Clone, Debug)]
    struct VrpSolution {
        routes: Vec<Route>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for VrpSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    impl VariableOperations for VrpSolution {
        type Element = usize;

        fn element_count(&self) -> usize {
            self.routes.iter().map(|r| r.stops.len()).sum()
        }

        fn entity_count(&self) -> usize {
            self.routes.len()
        }

        fn assigned_elements(&self) -> Vec<Self::Element> {
            self.routes
                .iter()
                .flat_map(|r| r.stops.iter().copied())
                .collect()
        }

        fn assign(&mut self, entity_idx: usize, elem: Self::Element) {
            self.routes[entity_idx].stops.push(elem);
        }

        fn list_len(&self, entity_idx: usize) -> usize {
            self.routes.get(entity_idx).map_or(0, |r| r.stops.len())
        }

        fn get(&self, entity_idx: usize, pos: usize) -> Self::Element {
            self.routes[entity_idx].stops[pos]
        }

        fn remove(&mut self, entity_idx: usize, pos: usize) -> Self::Element {
            self.routes[entity_idx].stops.remove(pos)
        }

        fn insert(&mut self, entity_idx: usize, pos: usize, elem: Self::Element) {
            self.routes[entity_idx].stops.insert(pos, elem);
        }

        fn descriptor_index() -> usize {
            0
        }

        fn variable_name() -> &'static str {
            "stops"
        }

        fn is_list_variable() -> bool {
            true
        }
    }

    fn create_director(
        routes: Vec<Vec<usize>>,
    ) -> SimpleScoreDirector<VrpSolution, impl Fn(&VrpSolution) -> SimpleScore> {
        let routes = routes.into_iter().map(|stops| Route { stops }).collect();
        let solution = VrpSolution {
            routes,
            score: None,
        };
        let descriptor = SolutionDescriptor::new("VrpSolution", TypeId::of::<VrpSolution>());
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn generates_list_ruin_moves() {
        let director = create_director(vec![vec![1, 2, 3, 4, 5]]);

        let selector =
            ListRuinMoveSelector::<VrpSolution>::new(2, 3, "stops", 0).with_moves_per_step(5);

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        assert_eq!(moves.len(), 5);
        for m in &moves {
            let count = m.ruin_count();
            assert!((2..=3).contains(&count));
        }
    }

    #[test]
    fn clamps_to_available_elements() {
        let director = create_director(vec![vec![1, 2]]);

        // Request more elements than available
        let selector =
            ListRuinMoveSelector::<VrpSolution>::new(5, 10, "stops", 0).with_moves_per_step(3);

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        assert_eq!(moves.len(), 3);
        for m in &moves {
            assert!(m.ruin_count() <= 2);
        }
    }

    #[test]
    fn empty_solution_yields_no_moves() {
        let director = create_director(vec![]);

        let selector = ListRuinMoveSelector::<VrpSolution>::new(1, 2, "stops", 0);

        let moves: Vec<_> = selector.iter_moves(&director).collect();
        assert!(moves.is_empty());
    }

    #[test]
    fn empty_list_yields_no_moves_for_that_entity() {
        let director = create_director(vec![vec![], vec![1, 2, 3]]);

        let selector = ListRuinMoveSelector::<VrpSolution>::new(1, 2, "stops", 0)
            .with_moves_per_step(10)
            .with_seed(42);

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // Some moves may be None due to empty list selection
        // All returned moves should be valid
        for m in &moves {
            assert!(m.ruin_count() >= 1);
        }
    }

    #[test]
    fn size_returns_moves_per_step() {
        let director = create_director(vec![vec![1, 2, 3]]);

        let selector =
            ListRuinMoveSelector::<VrpSolution>::new(1, 2, "stops", 0).with_moves_per_step(7);

        assert_eq!(selector.size(&director), 7);
    }

    #[test]
    #[should_panic(expected = "min_ruin_count must be at least 1")]
    fn panics_on_zero_min() {
        let _selector = ListRuinMoveSelector::<VrpSolution>::new(0, 2, "stops", 0);
    }

    #[test]
    #[should_panic(expected = "max_ruin_count must be >= min_ruin_count")]
    fn panics_on_invalid_range() {
        let _selector = ListRuinMoveSelector::<VrpSolution>::new(5, 2, "stops", 0);
    }
}
