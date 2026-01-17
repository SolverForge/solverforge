//! List change move selector for element relocation.
//!
//! Generates `ListChangeMove`s that relocate elements within or between list variables.
//! Essential for vehicle routing and scheduling problems.
//!
//! # Zero-Erasure Design
//!
//! No value type parameter. Uses VariableOperations trait for list access.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::list_change::ListChangeMove;
use crate::operations::VariableOperations;

use super::entity::EntitySelector;
use super::typed_move_selector::MoveSelector;

/// A move selector that generates list change moves.
///
/// Enumerates all valid (source_entity, source_pos, dest_entity, dest_pos)
/// combinations for relocating elements within or between list variables.
///
/// # Type Parameters
/// * `S` - The solution type (must implement VariableOperations)
/// * `ES` - The entity selector type
///
/// # Complexity
///
/// For n entities with average route length m:
/// - Intra-entity moves: O(n * m * m)
/// - Inter-entity moves: O(n * n * m * m)
/// - Total: O(n² * m²) worst case
///
/// Use with a forager that quits early for better performance.
pub struct ListChangeMoveSelector<S, ES> {
    /// Selects entities (vehicles) for moves.
    entity_selector: ES,
    /// Variable name for notifications.
    variable_name: &'static str,
    /// Entity descriptor index.
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, ES: Debug> Debug for ListChangeMoveSelector<S, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListChangeMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, ES> ListChangeMoveSelector<S, ES> {
    /// Creates a new list change move selector.
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

impl<S, ES> MoveSelector<S, ListChangeMove<S>> for ListChangeMoveSelector<S, ES>
where
    S: PlanningSolution + VariableOperations,
    ES: EntitySelector<S>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = ListChangeMove<S>> + 'a> {
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

        // Generate all valid moves
        let mut moves = Vec::new();

        for (src_idx, &src_entity) in entities.iter().enumerate() {
            let src_len = route_lens[src_idx];
            if src_len == 0 {
                continue;
            }

            for src_pos in 0..src_len {
                // Intra-entity moves
                for dst_pos in 0..src_len {
                    // Skip no-op moves:
                    // - Same position is obviously a no-op
                    // - Forward by 1 is a no-op due to index adjustment during do_move
                    if src_pos == dst_pos || dst_pos == src_pos + 1 {
                        continue;
                    }

                    moves.push(ListChangeMove::new(
                        src_entity,
                        src_pos,
                        src_entity,
                        dst_pos,
                        variable_name,
                        descriptor_index,
                    ));
                }

                // Inter-entity moves
                for (dst_idx, &dst_entity) in entities.iter().enumerate() {
                    if dst_idx == src_idx {
                        continue;
                    }

                    let dst_len = route_lens[dst_idx];
                    // Can insert at any position from 0 to dst_len inclusive
                    for dst_pos in 0..=dst_len {
                        moves.push(ListChangeMove::new(
                            src_entity,
                            src_pos,
                            dst_entity,
                            dst_pos,
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

        // Approximate: each element can move to any position in any entity
        // Intra: ~m positions per entity
        // Inter: ~(n-1) * m positions
        let n = entities.len();
        if n == 0 || total_elements == 0 {
            return 0;
        }

        let avg_len = total_elements / n;
        // Intra moves: n * m * m
        // Inter moves: n * (n-1) * m * m
        n * avg_len * (avg_len + (n - 1) * avg_len)
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
    fn generates_intra_entity_moves() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3],
        }];
        let director = create_director(vehicles);

        let selector = ListChangeMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // 3 elements. For each position, moves are generated to all other positions
        // EXCEPT forward by 1 (which is a no-op due to index adjustment).
        // From 0: skip 1 (forward by 1), to 2 → 1 move
        // From 1: to 0, skip 2 (forward by 1) → 1 move
        // From 2: to 0, to 1 → 2 moves
        // Total: 4 moves
        assert_eq!(moves.len(), 4);

        // All should be intra-list
        for m in &moves {
            assert!(m.is_intra_list());
        }
    }

    #[test]
    fn generates_inter_entity_moves() {
        let vehicles = vec![Vehicle { visits: vec![1, 2] }, Vehicle { visits: vec![10] }];
        let director = create_director(vehicles);

        let selector = ListChangeMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // Count inter-entity moves
        let inter_count = moves.iter().filter(|m| !m.is_intra_list()).count();
        // Vehicle 0 has 2 elements, each can go to vehicle 1 at positions 0,1 = 4 moves
        // Vehicle 1 has 1 element, can go to vehicle 0 at positions 0,1,2 = 3 moves
        assert_eq!(inter_count, 7);
    }

    #[test]
    fn moves_are_doable() {
        let vehicles = vec![
            Vehicle {
                visits: vec![1, 2, 3],
            },
            Vehicle { visits: vec![10] },
        ];
        let director = create_director(vehicles);

        let selector = ListChangeMoveSelector::<Solution, _>::new(
            FromSolutionEntitySelector::new(0),
            "visits",
            0,
        );

        for m in selector.iter_moves(&director) {
            assert!(m.is_doable(&director), "Move should be doable: {:?}", m);
        }
    }
}
