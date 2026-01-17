//! SubList swap move selector for segment exchanges.
//!
//! Generates `SubListSwapMove`s that swap contiguous segments
//! within or between list variables. Essential for VRP-style problems.
//!
//! # Zero-Erasure Design
//!
//! No value type parameter. Uses VariableOperations trait for list access.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::sublist_swap::SubListSwapMove;
use crate::operations::VariableOperations;

use super::entity::EntitySelector;
use super::typed_move_selector::MoveSelector;

/// A move selector that generates sublist swap moves.
///
/// Enumerates all valid pairs of sublists to swap within or between entities.
///
/// # Type Parameters
/// * `S` - The solution type (must implement VariableOperations)
/// * `ES` - The entity selector type
///
/// # Complexity
///
/// For n entities with average route length m:
/// - Number of sublists per entity: O(m²)
/// - Pairs of sublists: O(n² * m⁴) worst case
///
/// Use `min_sublist_len` and `max_sublist_len` to limit complexity.
pub struct SubListSwapMoveSelector<S, ES> {
    /// Selects entities for moves.
    entity_selector: ES,
    /// Variable name for notifications.
    variable_name: &'static str,
    /// Entity descriptor index.
    descriptor_index: usize,
    /// Minimum sublist length (inclusive).
    min_sublist_len: usize,
    /// Maximum sublist length (inclusive).
    max_sublist_len: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, ES: Debug> Debug for SubListSwapMoveSelector<S, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubListSwapMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .field("min_sublist_len", &self.min_sublist_len)
            .field("max_sublist_len", &self.max_sublist_len)
            .finish()
    }
}

impl<S, ES> SubListSwapMoveSelector<S, ES> {
    /// Creates a new sublist swap move selector.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to consider for moves
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    /// * `min_sublist_len` - Minimum sublist length (default: 1)
    /// * `max_sublist_len` - Maximum sublist length
    pub fn new(
        entity_selector: ES,
        variable_name: &'static str,
        descriptor_index: usize,
        min_sublist_len: usize,
        max_sublist_len: usize,
    ) -> Self {
        Self {
            entity_selector,
            variable_name,
            descriptor_index,
            min_sublist_len: min_sublist_len.max(1),
            max_sublist_len,
            _phantom: PhantomData,
        }
    }
}

/// Represents a sublist: (entity_idx, start, end)
type Sublist = (usize, usize, usize);

impl<S, ES> MoveSelector<S, SubListSwapMove<S>> for SubListSwapMoveSelector<S, ES>
where
    S: PlanningSolution + VariableOperations,
    ES: EntitySelector<S>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = SubListSwapMove<S>> + 'a> {
        let solution = score_director.working_solution();
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;
        let min_len = self.min_sublist_len;
        let max_len = self.max_sublist_len;

        // Collect entities
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

        // Generate all sublists for each entity
        let mut all_sublists: Vec<(usize, Vec<Sublist>)> = Vec::new();

        for (idx, &entity) in entities.iter().enumerate() {
            let route_len = route_lens[idx];
            if route_len < min_len {
                continue;
            }

            let mut sublists = Vec::new();
            for start in 0..route_len {
                let max_end = (start + max_len).min(route_len);
                for end in (start + min_len)..=max_end {
                    sublists.push((entity, start, end));
                }
            }

            if !sublists.is_empty() {
                all_sublists.push((idx, sublists));
            }
        }

        // Generate all valid swap moves
        let mut moves = Vec::new();

        for (entity_idx1, sublists1) in all_sublists.iter() {
            // Intra-entity swaps: only upper triangle to avoid duplicates
            for (i, &(e1, s1, e1_end)) in sublists1.iter().enumerate() {
                for &(_, s2, e2_end) in sublists1.iter().skip(i + 1) {
                    // Ranges must not overlap
                    let overlaps = s1 < e2_end && s2 < e1_end;
                    if overlaps {
                        continue;
                    }

                    moves.push(SubListSwapMove::new(
                        e1,
                        s1,
                        e1_end,
                        e1,
                        s2,
                        e2_end,
                        variable_name,
                        descriptor_index,
                    ));
                }
            }

            // Inter-entity swaps: only entity1 < entity2 to avoid duplicates
            for (entity_idx2, sublists2) in all_sublists.iter() {
                if entity_idx2 <= entity_idx1 {
                    continue;
                }

                for &(e1, s1, e1_end) in sublists1.iter() {
                    for &(e2, s2, e2_end) in sublists2.iter() {
                        moves.push(SubListSwapMove::new(
                            e1,
                            s1,
                            e1_end,
                            e2,
                            s2,
                            e2_end,
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

        // Rough approximation: O(n² * m⁴)
        let avg_len = total_elements / n;
        let sublists_per_entity = avg_len * avg_len / 2;
        sublists_per_entity * sublists_per_entity * n * n / 2
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
            self.vehicles
                .iter()
                .flat_map(|v| v.visits.iter().copied())
                .collect()
        }

        fn assign(&mut self, entity_idx: usize, elem: Self::Element) {
            self.vehicles[entity_idx].visits.push(elem);
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
            visits: vec![1, 2, 3, 4, 5, 6],
        }];
        let director = create_director(vehicles);

        let selector = SubListSwapMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
            1,
            2,
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // All should be intra-list
        for m in &moves {
            assert!(m.is_intra_list());
        }

        // Should have non-overlapping sublist swaps
        assert!(!moves.is_empty());
    }

    #[test]
    fn generates_inter_entity_swaps() {
        let vehicles = vec![
            Vehicle {
                visits: vec![1, 2, 3],
            },
            Vehicle {
                visits: vec![10, 20, 30],
            },
        ];
        let director = create_director(vehicles);

        let selector = SubListSwapMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
            1,
            2,
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // Count inter-entity swaps
        let inter_count = moves.iter().filter(|m| !m.is_intra_list()).count();
        assert!(inter_count > 0);
    }

    #[test]
    fn moves_are_doable() {
        let vehicles = vec![
            Vehicle {
                visits: vec![1, 2, 3, 4, 5],
            },
            Vehicle {
                visits: vec![10, 20, 30],
            },
        ];
        let director = create_director(vehicles);

        let selector = SubListSwapMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
            1,
            2,
        );

        for m in selector.iter_moves(&director) {
            assert!(m.is_doable(&director), "Move should be doable: {:?}", m);
        }
    }

    #[test]
    fn no_overlapping_intra_swaps() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3, 4, 5],
        }];
        let director = create_director(vehicles);

        let selector = SubListSwapMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
            1,
            3,
        );

        for m in selector.iter_moves(&director) {
            // For intra-list swaps, ranges should not overlap
            if m.is_intra_list() {
                let overlaps = m.first_start() < m.second_end() && m.second_start() < m.first_end();
                assert!(
                    !overlaps,
                    "Intra-list swap should not have overlapping ranges: {:?}",
                    m
                );
            }
        }
    }

    #[test]
    fn empty_entities_produce_no_moves() {
        let vehicles = vec![Vehicle { visits: vec![] }, Vehicle { visits: vec![] }];
        let director = create_director(vehicles);

        let selector = SubListSwapMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
            1,
            10,
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();
        assert!(moves.is_empty());
    }
}
