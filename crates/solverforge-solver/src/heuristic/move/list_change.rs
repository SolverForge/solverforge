//! ListChangeMove - relocates an element within or between list variables.
//!
//! This move removes an element from one position and inserts it at another.
//! Essential for vehicle routing and scheduling problems.
//!
//! # Zero-Erasure Design
//!
//! Stores only indices. No value type parameter. Operations use VariableOperations trait.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::operations::VariableOperations;

use super::Move;

/// A move that relocates an element from one list position to another.
///
/// Supports both intra-list moves (within same entity) and inter-list moves
/// (between different entities). Uses `VariableOperations` trait for zero-erasure.
///
/// # Type Parameters
/// * `S` - The planning solution type (must implement VariableOperations)
#[derive(Clone, Copy)]
pub struct ListChangeMove<S> {
    /// Source entity index (which entity's list to remove from)
    source_entity_index: usize,
    /// Position in source list to remove from
    source_position: usize,
    /// Destination entity index (which entity's list to insert into)
    dest_entity_index: usize,
    /// Position in destination list to insert at
    dest_position: usize,
    variable_name: &'static str,
    descriptor_index: usize,
    /// Store indices for entity_indices()
    indices: [usize; 2],
    _phantom: std::marker::PhantomData<fn() -> S>,
}

impl<S> Debug for ListChangeMove<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListChangeMove")
            .field("source_entity", &self.source_entity_index)
            .field("source_position", &self.source_position)
            .field("dest_entity", &self.dest_entity_index)
            .field("dest_position", &self.dest_position)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S> ListChangeMove<S> {
    /// Creates a new list change move.
    ///
    /// # Arguments
    /// * `source_entity_index` - Entity index to remove from
    /// * `source_position` - Position in source list
    /// * `dest_entity_index` - Entity index to insert into
    /// * `dest_position` - Position in destination list
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    pub fn new(
        source_entity_index: usize,
        source_position: usize,
        dest_entity_index: usize,
        dest_position: usize,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            source_entity_index,
            source_position,
            dest_entity_index,
            dest_position,
            variable_name,
            descriptor_index,
            indices: [source_entity_index, dest_entity_index],
            _phantom: std::marker::PhantomData,
        }
    }

    /// Returns the source entity index.
    pub fn source_entity_index(&self) -> usize {
        self.source_entity_index
    }

    /// Returns the source position.
    pub fn source_position(&self) -> usize {
        self.source_position
    }

    /// Returns the destination entity index.
    pub fn dest_entity_index(&self) -> usize {
        self.dest_entity_index
    }

    /// Returns the destination position.
    pub fn dest_position(&self) -> usize {
        self.dest_position
    }

    /// Returns true if this is an intra-list move (same entity).
    pub fn is_intra_list(&self) -> bool {
        self.source_entity_index == self.dest_entity_index
    }
}

impl<S> Move<S> for ListChangeMove<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();

        // Check source position is valid
        let source_len = solution.list_len(self.source_entity_index);
        if self.source_position >= source_len {
            return false;
        }

        // Check destination position is valid
        // For intra-list, dest can be 0..=len-1 (after removal)
        // For inter-list, dest can be 0..=len
        let dest_len = solution.list_len(self.dest_entity_index);
        let max_dest = if self.is_intra_list() {
            source_len - 1 // After removal, list is shorter
        } else {
            dest_len
        };

        if self.dest_position > max_dest {
            return false;
        }

        // For intra-list moves, check for no-ops
        if self.is_intra_list() {
            if self.source_position == self.dest_position {
                return false;
            }
            // Forward move by exactly 1 is a no-op
            if self.dest_position == self.source_position + 1 {
                return false;
            }
        }

        true
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        // Notify before changes
        score_director.before_variable_changed(
            self.descriptor_index,
            self.source_entity_index,
            self.variable_name,
        );
        if !self.is_intra_list() {
            score_director.before_variable_changed(
                self.descriptor_index,
                self.dest_entity_index,
                self.variable_name,
            );
        }

        // Remove from source
        let value = score_director
            .working_solution_mut()
            .remove(self.source_entity_index, self.source_position);

        // For intra-list moves, adjust dest position if it was after source
        let adjusted_dest = if self.is_intra_list() && self.dest_position > self.source_position {
            self.dest_position - 1
        } else {
            self.dest_position
        };

        // Insert at destination
        score_director
            .working_solution_mut()
            .insert(self.dest_entity_index, adjusted_dest, value);

        // Notify after changes
        score_director.after_variable_changed(
            self.descriptor_index,
            self.source_entity_index,
            self.variable_name,
        );
        if !self.is_intra_list() {
            score_director.after_variable_changed(
                self.descriptor_index,
                self.dest_entity_index,
                self.variable_name,
            );
        }

        // Register undo - reverse the operation
        let src_entity = self.source_entity_index;
        let src_pos = self.source_position;
        let dest_entity = self.dest_entity_index;
        let is_intra = self.is_intra_list();

        score_director.register_undo(Box::new(move |s: &mut S| {
            // Remove from where we inserted
            let removed = s.remove(dest_entity, adjusted_dest);
            // Insert back at original source position
            // For intra-list backward moves, need to adjust
            let restore_pos = if is_intra && src_pos > adjusted_dest {
                src_pos
            } else {
                src_pos
            };
            s.insert(src_entity, restore_pos, removed);
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        if self.is_intra_list() {
            &self.indices[0..1]
        } else {
            &self.indices
        }
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::{RecordingScoreDirector, SimpleScoreDirector};
    use std::any::TypeId;
    use std::hash::Hash;

    #[derive(Clone, Debug)]
    struct Vehicle {
        visits: Vec<usize>,
    }

    #[derive(Clone, Debug)]
    struct RoutingSolution {
        vehicles: Vec<Vehicle>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for RoutingSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    impl VariableOperations for RoutingSolution {
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

        fn assign(&mut self, entity_idx: usize, elem: Self::Element) {
            self.vehicles[entity_idx].visits.push(elem);
        }

        fn list_len(&self, entity_idx: usize) -> usize {
            self.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
        }

        fn remove(&mut self, entity_idx: usize, pos: usize) -> Self::Element {
            self.vehicles[entity_idx].visits.remove(pos)
        }

        fn insert(&mut self, entity_idx: usize, pos: usize, elem: Self::Element) {
            self.vehicles[entity_idx].visits.insert(pos, elem);
        }

        fn get(&self, entity_idx: usize, pos: usize) -> Self::Element {
            self.vehicles[entity_idx].visits[pos]
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

    fn get_vehicles(s: &RoutingSolution) -> &Vec<Vehicle> {
        &s.vehicles
    }
    fn get_vehicles_mut(s: &mut RoutingSolution) -> &mut Vec<Vehicle> {
        &mut s.vehicles
    }

    fn create_director(
        vehicles: Vec<Vehicle>,
    ) -> SimpleScoreDirector<RoutingSolution, impl Fn(&RoutingSolution) -> SimpleScore> {
        let solution = RoutingSolution {
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
            SolutionDescriptor::new("RoutingSolution", TypeId::of::<RoutingSolution>())
                .with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn intra_list_move_forward() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3, 4, 5],
        }];
        let mut director = create_director(vehicles);

        // Move element at position 1 (value=2) to position 3
        let m = ListChangeMove::<RoutingSolution>::new(0, 1, 0, 3, "visits", 0);

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            // After: [1, 3, 2, 4, 5] (2 moved to after 3)
            let visits = &recording.working_solution().vehicles[0].visits;
            assert_eq!(visits, &[1, 3, 2, 4, 5]);

            recording.undo_changes();
        }

        // Verify restored
        let visits = &director.working_solution().vehicles[0].visits;
        assert_eq!(visits, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn intra_list_move_backward() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3, 4, 5],
        }];
        let mut director = create_director(vehicles);

        // Move element at position 3 (value=4) to position 1
        let m = ListChangeMove::<RoutingSolution>::new(0, 3, 0, 1, "visits", 0);

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            // After: [1, 4, 2, 3, 5]
            let visits = &recording.working_solution().vehicles[0].visits;
            assert_eq!(visits, &[1, 4, 2, 3, 5]);

            recording.undo_changes();
        }

        let visits = &director.working_solution().vehicles[0].visits;
        assert_eq!(visits, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn inter_list_move() {
        let vehicles = vec![
            Vehicle {
                visits: vec![1, 2, 3],
            },
            Vehicle {
                visits: vec![10, 20],
            },
        ];
        let mut director = create_director(vehicles);

        // Move element from vehicle 0 position 1 (value=2) to vehicle 1 position 1
        let m = ListChangeMove::<RoutingSolution>::new(0, 1, 1, 1, "visits", 0);

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let sol = recording.working_solution();
            assert_eq!(sol.vehicles[0].visits, vec![1, 3]);
            assert_eq!(sol.vehicles[1].visits, vec![10, 2, 20]);

            recording.undo_changes();
        }

        let sol = director.working_solution();
        assert_eq!(sol.vehicles[0].visits, vec![1, 2, 3]);
        assert_eq!(sol.vehicles[1].visits, vec![10, 20]);
    }

    #[test]
    fn same_position_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3],
        }];
        let director = create_director(vehicles);

        // Move to same effective position
        let m = ListChangeMove::<RoutingSolution>::new(0, 1, 0, 1, "visits", 0);

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn invalid_source_position_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3],
        }];
        let director = create_director(vehicles);

        let m = ListChangeMove::<RoutingSolution>::new(0, 10, 0, 0, "visits", 0);

        assert!(!m.is_doable(&director));
    }
}
