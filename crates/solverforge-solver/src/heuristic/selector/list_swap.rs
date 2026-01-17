//! List swap move selector for element exchanges.
//!
//! Generates `ListSwapMove`s that swap two elements within or between list variables.
//! Useful for TSP-style improvements and route optimization.
//!
//! # Zero-Erasure Design
//!
//! No value type parameter. Uses VariableOperations trait for list access.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::list_swap::ListSwapMove;
use crate::operations::VariableOperations;

use super::entity::EntitySelector;
use super::typed_move_selector::MoveSelector;

/// A move selector that generates list swap moves.
///
/// Enumerates all valid (entity1, pos1, entity2, pos2) pairs for swapping
/// elements within or between list variables.
///
/// # Type Parameters
/// * `S` - The solution type (must implement VariableOperations)
/// * `ES` - The entity selector type
///
/// # Complexity
///
/// For n entities with average route length m:
/// - Intra-entity swaps: O(n * m²/2) (triangle numbers)
/// - Inter-entity swaps: O(n² * m²/2)
/// - Total: O(n² * m²)
///
/// Use with a forager that quits early for better performance.
pub struct ListSwapMoveSelector<S, ES> {
    /// Selects entities for moves.
    entity_selector: ES,
    /// Variable name for notifications.
    variable_name: &'static str,
    /// Entity descriptor index.
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, ES: Debug> Debug for ListSwapMoveSelector<S, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListSwapMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, ES> ListSwapMoveSelector<S, ES> {
    /// Creates a new list swap move selector.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to consider for moves
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    pub fn new(entity_selector: ES, variable_name: &'static str, descriptor_index: usize) -> Self {
        Self {
            entity_selector,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, ES> MoveSelector<S, ListSwapMove<S>> for ListSwapMoveSelector<S, ES>
where
    S: PlanningSolution + VariableOperations,
    ES: EntitySelector<S>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = ListSwapMove<S>> + 'a> {
        let solution = score_director.working_solution();
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;

        // Collect entities to allow multiple passes
        let entities: Vec<usize> = self
            .entity_selector
            .iter(score_director)
            .map(|r| r.entity_index)
            .collect();

        // Pre-compute route lengths
        let route_lens: Vec<usize> = entities
            .iter()
            .map(|&e| solution.list_len(e))
            .collect();

        // Generate all valid swap moves
        let mut moves = Vec::new();

        for (idx1, &entity1) in entities.iter().enumerate() {
            let len1 = route_lens[idx1];
            if len1 == 0 {
                continue;
            }

            // Intra-entity swaps: only upper triangle (pos1 < pos2) to avoid duplicates
            for pos1 in 0..len1 {
                for pos2 in (pos1 + 1)..len1 {
                    moves.push(ListSwapMove::new(
                        entity1,
                        pos1,
                        entity1,
                        pos2,
                        variable_name,
                        descriptor_index,
                    ));
                }
            }

            // Inter-entity swaps: only entity1 < entity2 to avoid duplicate swaps
            for (idx2, &entity2) in entities.iter().enumerate().skip(idx1 + 1) {
                let len2 = route_lens[idx2];
                if len2 == 0 {
                    continue;
                }

                for pos1 in 0..len1 {
                    for pos2 in 0..len2 {
                        moves.push(ListSwapMove::new(
                            entity1,
                            pos1,
                            entity2,
                            pos2,
                            variable_name,
                            descriptor_index,
                        ));
                    }
                }
            }
        }

        Box::new(moves.into_iter())
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        let solution = score_director.working_solution();

        let entities: Vec<usize> = self
            .entity_selector
            .iter(score_director)
            .map(|r| r.entity_index)
            .collect();

        let route_lens: Vec<usize> = entities
            .iter()
            .map(|&e| solution.list_len(e))
            .collect();
        let total_elements: usize = route_lens.iter().sum();

        let n = entities.len();
        if n == 0 || total_elements == 0 {
            return 0;
        }

        // Intra-entity: sum of triangle numbers = n * m * (m-1) / 2
        // Inter-entity: n * (n-1) * m² / 2
        let avg_len = total_elements / n;
        let intra = n * avg_len * avg_len.saturating_sub(1) / 2;
        let inter = n * n.saturating_sub(1) * avg_len * avg_len / 2;
        intra + inter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::r#move::Move;
    use crate::heuristic::selector::entity::FromSolutionEntitySelector;
    use crate::operations::VariableOperations;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::SimpleScoreDirector;
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Vehicle {
        visits: Vec<usize>,
    }

    #[derive(Clone, Debug)]
    struct Solution {
        vehicles: Vec<Vehicle>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for Solution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    impl VariableOperations for Solution {
        type Element = usize;

        fn element_count(&self) -> usize {
            self.vehicles.iter().map(|v| v.visits.len()).sum()
        }

        fn entity_count(&self) -> usize {
            self.vehicles.len()
        }

        fn assigned_elements(&self) -> Vec<Self::Element> {
            self.vehicles.iter().flat_map(|v| v.visits.iter().copied()).collect()
        }

        fn assign(&mut self, _entity_idx: usize, _elem: Self::Element) {
            // Not used for list variables
        }

        fn list_len(&self, entity_idx: usize) -> usize {
            self.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
        }

        fn get(&self, entity_idx: usize, pos: usize) -> Self::Element {
            self.vehicles[entity_idx].visits[pos]
        }

        fn remove(&mut self, entity_idx: usize, pos: usize) -> Self::Element {
            self.vehicles[entity_idx].visits.remove(pos)
        }

        fn insert(&mut self, entity_idx: usize, pos: usize, elem: Self::Element) {
            self.vehicles[entity_idx].visits.insert(pos, elem);
        }

        fn descriptor_index() -> usize {
            0
        }

        fn variable_name() -> &'static str {
            "visits"
        }

        fn is_list_variable() -> bool {
            true
        }
    }

    fn get_vehicles(s: &Solution) -> &Vec<Vehicle> {
        &s.vehicles
    }
    fn get_vehicles_mut(s: &mut Solution) -> &mut Vec<Vehicle> {
        &mut s.vehicles
    }

    fn create_director(
        vehicles: Vec<Vehicle>,
    ) -> SimpleScoreDirector<Solution, impl Fn(&Solution) -> SimpleScore> {
        let solution = Solution {
            vehicles,
            score: None,
        };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Vehicle",
            "vehicles",
            get_vehicles,
            get_vehicles_mut,
        ));
        let entity_desc = EntityDescriptor::new("Vehicle", TypeId::of::<Vehicle>(), "vehicles")
            .with_extractor(extractor);
        let descriptor =
            SolutionDescriptor::new("Solution", TypeId::of::<Solution>()).with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn generates_intra_entity_swaps() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3, 4],
        }];
        let director = create_director(vehicles);

        let selector = ListSwapMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // 4 elements → C(4,2) = 6 swaps
        assert_eq!(moves.len(), 6);

        // All should be intra-list
        for m in &moves {
            assert!(m.is_intra_list());
        }
    }

    #[test]
    fn generates_inter_entity_swaps() {
        let vehicles = vec![
            Vehicle { visits: vec![1, 2] },
            Vehicle {
                visits: vec![10, 20, 30],
            },
        ];
        let director = create_director(vehicles);

        let selector = ListSwapMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // Intra-entity:
        //   Vehicle 0: C(2,2) = 1 swap
        //   Vehicle 1: C(3,2) = 3 swaps
        // Inter-entity: 2 * 3 = 6 swaps
        // Total: 1 + 3 + 6 = 10
        assert_eq!(moves.len(), 10);

        // Count inter-entity swaps
        let inter_count = moves.iter().filter(|m| !m.is_intra_list()).count();
        assert_eq!(inter_count, 6);
    }

    #[test]
    fn moves_are_doable() {
        let vehicles = vec![
            Vehicle {
                visits: vec![1, 2, 3],
            },
            Vehicle {
                visits: vec![10, 20],
            },
        ];
        let director = create_director(vehicles);

        let selector = ListSwapMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
        );

        for m in selector.iter_moves(&director) {
            assert!(m.is_doable(&director), "Move should be doable: {:?}", m);
        }
    }

    #[test]
    fn empty_entities_produce_no_moves() {
        let vehicles = vec![Vehicle { visits: vec![] }, Vehicle { visits: vec![] }];
        let director = create_director(vehicles);

        let selector = ListSwapMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();
        assert!(moves.is_empty());
    }

    #[test]
    fn single_element_lists_inter_only() {
        let vehicles = vec![
            Vehicle { visits: vec![1] },
            Vehicle { visits: vec![2] },
            Vehicle { visits: vec![3] },
        ];
        let director = create_director(vehicles);

        let selector = ListSwapMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // No intra-entity swaps (need 2+ elements)
        // Inter-entity: C(3,2) = 3 pairs, 1*1 positions each = 3 swaps
        assert_eq!(moves.len(), 3);

        // All should be inter-list
        for m in &moves {
            assert!(!m.is_intra_list());
        }
    }
}
