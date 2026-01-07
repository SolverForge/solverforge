//! List change move selector for element relocation.
//!
//! Generates `ListChangeMove`s that relocate elements within or between list variables.
//! Essential for vehicle routing and scheduling problems.
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::selector::list_change::ListChangeMoveSelector;
//! use solverforge_solver::heuristic::selector::entity::FromSolutionEntitySelector;
//! use solverforge_solver::heuristic::selector::MoveSelector;
//! use solverforge_core::domain::PlanningSolution;
//! use solverforge_core::score::SimpleScore;
//!
//! #[derive(Clone, Debug)]
//! struct Vehicle { visits: Vec<i32> }
//!
//! #[derive(Clone, Debug)]
//! struct Solution { vehicles: Vec<Vehicle>, score: Option<SimpleScore> }
//!
//! impl PlanningSolution for Solution {
//!     type Score = SimpleScore;
//!     fn score(&self) -> Option<Self::Score> { self.score }
//!     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
//! }
//!
//! fn list_len(s: &Solution, entity_idx: usize) -> usize {
//!     s.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
//! }
//! fn list_remove(s: &mut Solution, entity_idx: usize, pos: usize) -> Option<i32> {
//!     s.vehicles.get_mut(entity_idx).map(|v| v.visits.remove(pos))
//! }
//! fn list_insert(s: &mut Solution, entity_idx: usize, pos: usize, val: i32) {
//!     if let Some(v) = s.vehicles.get_mut(entity_idx) { v.visits.insert(pos, val); }
//! }
//!
//! let selector = ListChangeMoveSelector::<Solution, i32>::new(
//!     Box::new(FromSolutionEntitySelector::new(0)),
//!     list_len,
//!     list_remove,
//!     list_insert,
//!     "visits",
//!     0,
//! );
//! ```

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::ListChangeMove;

use super::entity::EntitySelector;
use super::typed_move_selector::MoveSelector;

/// A move selector that generates list change moves.
///
/// Enumerates all valid (source_entity, source_pos, dest_entity, dest_pos)
/// combinations for relocating elements within or between list variables.
///
/// # Type Parameters
/// * `S` - The solution type
/// * `V` - The list element type
///
/// # Complexity
///
/// For n entities with average route length m:
/// - Intra-entity moves: O(n * m * m)
/// - Inter-entity moves: O(n * n * m * m)
/// - Total: O(n² * m²) worst case
///
/// Use with a forager that quits early for better performance.
pub struct ListChangeMoveSelector<S, V> {
    /// Selects entities (vehicles) for moves.
    entity_selector: Box<dyn EntitySelector<S>>,
    /// Get list length for an entity.
    list_len: fn(&S, usize) -> usize,
    /// Remove element at position.
    list_remove: fn(&mut S, usize, usize) -> Option<V>,
    /// Insert element at position.
    list_insert: fn(&mut S, usize, usize, V),
    /// Variable name for notifications.
    variable_name: &'static str,
    /// Entity descriptor index.
    descriptor_index: usize,
    _phantom: PhantomData<V>,
}

impl<S, V: Debug> Debug for ListChangeMoveSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListChangeMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V> ListChangeMoveSelector<S, V> {
    /// Creates a new list change move selector.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to consider for moves
    /// * `list_len` - Function to get list length for an entity
    /// * `list_remove` - Function to remove element at position
    /// * `list_insert` - Function to insert element at position
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    pub fn new(
        entity_selector: Box<dyn EntitySelector<S>>,
        list_len: fn(&S, usize) -> usize,
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        list_insert: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            list_len,
            list_remove,
            list_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> MoveSelector<S, ListChangeMove<S, V>> for ListChangeMoveSelector<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn iter_moves<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
    ) -> Box<dyn Iterator<Item = ListChangeMove<S, V>> + 'a> {
        let solution = score_director.working_solution();
        let list_len = self.list_len;
        let list_remove = self.list_remove;
        let list_insert = self.list_insert;
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;

        // Collect entities to allow multiple passes
        let entities: Vec<usize> = self
            .entity_selector
            .iter(score_director)
            .map(|r| r.entity_index)
            .collect();

        // Pre-compute route lengths
        let route_lens: Vec<usize> = entities.iter().map(|&e| list_len(solution, e)).collect();

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
                    // Skip no-op moves (remove and insert at same position)
                    if src_pos == dst_pos {
                        continue;
                    }

                    moves.push(ListChangeMove::new(
                        src_entity,
                        src_pos,
                        src_entity,
                        dst_pos,
                        list_len,
                        list_remove,
                        list_insert,
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
                            list_len,
                            list_remove,
                            list_insert,
                            variable_name,
                            descriptor_index,
                        ));
                    }
                }
            }
        }

        Box::new(moves.into_iter())
    }

    fn size(&self, score_director: &dyn ScoreDirector<S>) -> usize {
        let solution = score_director.working_solution();
        let list_len = self.list_len;

        let entities: Vec<usize> = self
            .entity_selector
            .iter(score_director)
            .map(|r| r.entity_index)
            .collect();

        let route_lens: Vec<usize> = entities.iter().map(|&e| list_len(solution, e)).collect();
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
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::SimpleScoreDirector;
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Vehicle {
        visits: Vec<i32>,
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

    fn get_vehicles(s: &Solution) -> &Vec<Vehicle> {
        &s.vehicles
    }
    fn get_vehicles_mut(s: &mut Solution) -> &mut Vec<Vehicle> {
        &mut s.vehicles
    }

    fn list_len(s: &Solution, entity_idx: usize) -> usize {
        s.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
    }
    fn list_remove(s: &mut Solution, entity_idx: usize, pos: usize) -> Option<i32> {
        s.vehicles.get_mut(entity_idx).map(|v| v.visits.remove(pos))
    }
    fn list_insert(s: &mut Solution, entity_idx: usize, pos: usize, val: i32) {
        if let Some(v) = s.vehicles.get_mut(entity_idx) {
            v.visits.insert(pos, val);
        }
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
        let descriptor = SolutionDescriptor::new("Solution", TypeId::of::<Solution>())
            .with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn generates_intra_entity_moves() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3],
        }];
        let director = create_director(vehicles);

        let selector = ListChangeMoveSelector::<Solution, i32>::new(
            Box::new(FromSolutionEntitySelector::new(0)),
            list_len,
            list_remove,
            list_insert,
            "visits",
            0,
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // 3 elements, each can move to 2 other effective positions
        // (0->1, 0->2, 1->0, 1->2, 2->0, 2->1) = 6 moves
        assert_eq!(moves.len(), 6);

        // All should be intra-list
        for m in &moves {
            assert!(m.is_intra_list());
        }
    }

    #[test]
    fn generates_inter_entity_moves() {
        let vehicles = vec![
            Vehicle { visits: vec![1, 2] },
            Vehicle { visits: vec![10] },
        ];
        let director = create_director(vehicles);

        let selector = ListChangeMoveSelector::<Solution, i32>::new(
            Box::new(FromSolutionEntitySelector::new(0)),
            list_len,
            list_remove,
            list_insert,
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

        let selector = ListChangeMoveSelector::<Solution, i32>::new(
            Box::new(FromSolutionEntitySelector::new(0)),
            list_len,
            list_remove,
            list_insert,
            "visits",
            0,
        );

        for m in selector.iter_moves(&director) {
            assert!(m.is_doable(&director), "Move should be doable: {:?}", m);
        }
    }
}
