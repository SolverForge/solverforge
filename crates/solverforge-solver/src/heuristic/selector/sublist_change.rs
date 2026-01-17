//! SubList change move selector for segment relocation.
//!
//! Generates `SubListChangeMove`s that relocate contiguous segments
//! within or between list variables. Essential for VRP-style problems.
//!
//! # Zero-Erasure Design
//!
//! No value type parameter. Uses VariableOperations trait for list access.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::sublist_change::SubListChangeMove;
use crate::operations::VariableOperations;

use super::entity::EntitySelector;
use super::typed_move_selector::MoveSelector;

/// A move selector that generates sublist change moves.
///
/// Enumerates all valid (src_entity, src_range, dest_entity, dest_pos)
/// combinations for relocating contiguous segments.
///
/// # Type Parameters
/// * `S` - The solution type (must implement VariableOperations)
/// * `ES` - The entity selector type
///
/// # Complexity
///
/// For n entities with average route length m:
/// - Number of sublists per entity: O(m²)
/// - Number of destination positions: O(n * m)
/// - Total: O(n² * m³) worst case
///
/// Use `min_sublist_len` and `max_sublist_len` to limit complexity.
pub struct SubListChangeMoveSelector<S, ES> {
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

impl<S, ES: Debug> Debug for SubListChangeMoveSelector<S, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubListChangeMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .field("min_sublist_len", &self.min_sublist_len)
            .field("max_sublist_len", &self.max_sublist_len)
            .finish()
    }
}

impl<S, ES> SubListChangeMoveSelector<S, ES> {
    /// Creates a new sublist change move selector.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to consider for moves
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    /// * `min_sublist_len` - Minimum sublist length (default: 1)
    /// * `max_sublist_len` - Maximum sublist length (default: unlimited)
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

impl<S, ES> MoveSelector<S, SubListChangeMove<S>> for SubListChangeMoveSelector<S, ES>
where
    S: PlanningSolution + VariableOperations,
    ES: EntitySelector<S>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = SubListChangeMove<S>> + 'a> {
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

        // Generate all valid moves
        let mut moves = Vec::new();

        for (src_idx, &src_entity) in entities.iter().enumerate() {
            let src_len = route_lens[src_idx];
            if src_len < min_len {
                continue;
            }

            // Generate all sublists from this entity
            for start in 0..src_len {
                let max_sublist_end = (start + max_len).min(src_len);

                for end in (start + min_len)..=max_sublist_end {
                    let sublist_len = end - start;

                    // Intra-entity moves: after removing [start, end), list is shorter
                    let post_removal_len = src_len - sublist_len;

                    for dest_pos in 0..=post_removal_len {
                        // Skip if destination is within source range (no-op)
                        if dest_pos >= start && dest_pos <= end {
                            continue;
                        }

                        moves.push(SubListChangeMove::new(
                            src_entity,
                            start,
                            end,
                            src_entity,
                            dest_pos,
                            variable_name,
                            descriptor_index,
                        ));
                    }

                    // Inter-entity moves
                    for (dest_idx, &dest_entity) in entities.iter().enumerate() {
                        if dest_idx == src_idx {
                            continue;
                        }

                        let dest_len = route_lens[dest_idx];

                        for dest_pos in 0..=dest_len {
                            moves.push(SubListChangeMove::new(
                                src_entity,
                                start,
                                end,
                                dest_entity,
                                dest_pos,
                                variable_name,
                                descriptor_index,
                            ));
                        }
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

        // Rough approximation: O(n² * m³)
        let avg_len = total_elements / n;
        let sublists_per_entity = avg_len * avg_len / 2; // triangle number approx
        let dest_positions = n * avg_len;
        sublists_per_entity * dest_positions
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
    fn generates_intra_entity_moves() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3, 4],
        }];
        let director = create_director(vehicles);

        let selector = SubListChangeMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
            1,
            2, // only sublists of length 1-2
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // All should be intra-list (only one entity)
        for m in &moves {
            assert!(m.is_intra_list());
        }

        // Should have some moves
        assert!(!moves.is_empty());
    }

    #[test]
    fn generates_inter_entity_moves() {
        let vehicles = vec![
            Vehicle {
                visits: vec![1, 2, 3],
            },
            Vehicle {
                visits: vec![10, 20],
            },
        ];
        let director = create_director(vehicles);

        let selector = SubListChangeMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
            1,
            2,
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // Count inter-entity moves
        let inter_count = moves.iter().filter(|m| !m.is_intra_list()).count();
        assert!(inter_count > 0);
    }

    #[test]
    fn moves_are_doable() {
        let vehicles = vec![
            Vehicle {
                visits: vec![1, 2, 3, 4],
            },
            Vehicle {
                visits: vec![10, 20],
            },
        ];
        let director = create_director(vehicles);

        let selector = SubListChangeMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
            1,
            3,
        );

        for m in selector.iter_moves(&director) {
            assert!(m.is_doable(&director), "Move should be doable: {:?}", m);
        }
    }

    #[test]
    fn respects_min_max_sublist_len() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3, 4, 5],
        }];
        let director = create_director(vehicles);

        let selector = SubListChangeMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
            2, // min length 2
            3, // max length 3
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // All sublists should be length 2 or 3
        for m in &moves {
            let len = m.sublist_len();
            assert!(
                len >= 2 && len <= 3,
                "Sublist length {} out of range [2, 3]",
                len
            );
        }
    }

    #[test]
    fn empty_entities_produce_no_moves() {
        let vehicles = vec![Vehicle { visits: vec![] }, Vehicle { visits: vec![] }];
        let director = create_director(vehicles);

        let selector = SubListChangeMoveSelector::<Solution, _>::new(
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
