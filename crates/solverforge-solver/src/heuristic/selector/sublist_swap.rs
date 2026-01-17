//! SubList swap move selector for segment exchanges.
//!
//! Generates `SubListSwapMove`s that swap contiguous segments
//! within or between list variables. Essential for VRP-style problems.
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::selector::sublist_swap::SubListSwapMoveSelector;
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
//! fn sublist_remove(s: &mut Solution, entity_idx: usize, start: usize, end: usize) -> Vec<i32> {
//!     s.vehicles.get_mut(entity_idx)
//!         .map(|v| v.visits.drain(start..end).collect())
//!         .unwrap_or_default()
//! }
//! fn sublist_insert(s: &mut Solution, entity_idx: usize, pos: usize, items: Vec<i32>) {
//!     if let Some(v) = s.vehicles.get_mut(entity_idx) {
//!         for (i, item) in items.into_iter().enumerate() {
//!             v.visits.insert(pos + i, item);
//!         }
//!     }
//! }
//!
//! let selector = SubListSwapMoveSelector::<Solution, i32, _>::new(
//!     FromSolutionEntitySelector::new(0),
//!     list_len,
//!     sublist_remove,
//!     sublist_insert,
//!     "visits",
//!     0,
//!     1,  // min sublist size
//!     3,  // max sublist size
//! );
//! ```

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::sublist_swap::SubListSwapMove;

use super::entity::EntitySelector;
use super::typed_move_selector::MoveSelector;

/// A move selector that generates sublist swap moves.
///
/// Enumerates all valid pairs of sublists to swap within or between entities.
///
/// # Type Parameters
/// * `S` - The solution type
/// * `V` - The list element type
/// * `ES` - The entity selector type
///
/// # Complexity
///
/// For n entities with average route length m:
/// - Number of sublists per entity: O(m²)
/// - Pairs of sublists: O(n² * m⁴) worst case
///
/// Use `min_sublist_len` and `max_sublist_len` to limit complexity.
pub struct SubListSwapMoveSelector<S, V, ES> {
    /// Selects entities for moves.
    entity_selector: ES,
    /// Get list length for an entity.
    list_len: fn(&S, usize) -> usize,
    /// Remove sublist [start, end).
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    /// Insert sublist at position.
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    /// Variable name for notifications.
    variable_name: &'static str,
    /// Entity descriptor index.
    descriptor_index: usize,
    /// Minimum sublist length (inclusive).
    min_sublist_len: usize,
    /// Maximum sublist length (inclusive).
    max_sublist_len: usize,
    _phantom: PhantomData<(S, V)>,
}

impl<S, V: Debug, ES: Debug> Debug for SubListSwapMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubListSwapMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .field("min_sublist_len", &self.min_sublist_len)
            .field("max_sublist_len", &self.max_sublist_len)
            .finish()
    }
}

impl<S, V, ES> SubListSwapMoveSelector<S, V, ES> {
    /// Creates a new sublist swap move selector.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to consider for moves
    /// * `list_len` - Function to get list length for an entity
    /// * `sublist_remove` - Function to remove sublist [start, end)
    /// * `sublist_insert` - Function to insert sublist at position
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    /// * `min_sublist_len` - Minimum sublist length (default: 1)
    /// * `max_sublist_len` - Maximum sublist length
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        list_len: fn(&S, usize) -> usize,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
        min_sublist_len: usize,
        max_sublist_len: usize,
    ) -> Self {
        Self {
            entity_selector,
            list_len,
            sublist_remove,
            sublist_insert,
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

impl<S, V, ES> MoveSelector<S, SubListSwapMove<S, V>> for SubListSwapMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = SubListSwapMove<S, V>> + 'a> {
        let solution = score_director.working_solution();
        let list_len = self.list_len;
        let sublist_remove = self.sublist_remove;
        let sublist_insert = self.sublist_insert;
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
        let route_lens: Vec<usize> = entities.iter().map(|&e| list_len(solution, e)).collect();

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
                        list_len,
                        sublist_remove,
                        sublist_insert,
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
                            list_len,
                            sublist_remove,
                            sublist_insert,
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
        let list_len = self.list_len;

        let entities: Vec<usize> = self
            .entity_selector
            .iter(score_director)
            .map(|r| r.entity_index)
            .collect();

        let route_lens: Vec<usize> = entities.iter().map(|&e| list_len(solution, e)).collect();
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
    fn sublist_remove(s: &mut Solution, entity_idx: usize, start: usize, end: usize) -> Vec<i32> {
        s.vehicles
            .get_mut(entity_idx)
            .map(|v| v.visits.drain(start..end).collect())
            .unwrap_or_default()
    }
    fn sublist_insert(s: &mut Solution, entity_idx: usize, pos: usize, items: Vec<i32>) {
        if let Some(v) = s.vehicles.get_mut(entity_idx) {
            for (i, item) in items.into_iter().enumerate() {
                v.visits.insert(pos + i, item);
            }
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

        let selector = SubListSwapMoveSelector::<Solution, i32, _>::new(
            FromSolutionEntitySelector::new(0),
            list_len,
            sublist_remove,
            sublist_insert,
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

        let selector = SubListSwapMoveSelector::<Solution, i32, _>::new(
            FromSolutionEntitySelector::new(0),
            list_len,
            sublist_remove,
            sublist_insert,
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

        let selector = SubListSwapMoveSelector::<Solution, i32, _>::new(
            FromSolutionEntitySelector::new(0),
            list_len,
            sublist_remove,
            sublist_insert,
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

        let selector = SubListSwapMoveSelector::<Solution, i32, _>::new(
            FromSolutionEntitySelector::new(0),
            list_len,
            sublist_remove,
            sublist_insert,
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

        let selector = SubListSwapMoveSelector::<Solution, i32, _>::new(
            FromSolutionEntitySelector::new(0),
            list_len,
            sublist_remove,
            sublist_insert,
            "visits",
            0,
            1,
            10,
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();
        assert!(moves.is_empty());
    }
}
