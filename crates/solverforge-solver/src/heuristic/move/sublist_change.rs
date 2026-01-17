//! SubListChangeMove - relocates a contiguous sublist within or between list variables.
//!
//! This move removes a range of elements from one position and inserts them at another.
//! Essential for vehicle routing where multiple consecutive stops need relocation.
//!
//! # Zero-Erasure Design
//!
//! Stores only indices. No value type parameter. Operations use VariableOperations trait.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::operations::VariableOperations;

use super::Move;

/// A move that relocates a contiguous sublist from one position to another.
///
/// Supports both intra-list moves (within same entity) and inter-list moves
/// (between different entities). Uses `VariableOperations` trait for zero-erasure.
///
/// # Type Parameters
/// * `S` - The planning solution type (must implement VariableOperations)
#[derive(Clone, Copy)]
pub struct SubListChangeMove<S> {
    /// Source entity index
    source_entity_index: usize,
    /// Start of range in source list (inclusive)
    source_start: usize,
    /// End of range in source list (exclusive)
    source_end: usize,
    /// Destination entity index
    dest_entity_index: usize,
    /// Position in destination list to insert at
    dest_position: usize,
    variable_name: &'static str,
    descriptor_index: usize,
    /// Store indices for entity_indices()
    indices: [usize; 2],
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for SubListChangeMove<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubListChangeMove")
            .field("source_entity", &self.source_entity_index)
            .field("source_range", &(self.source_start..self.source_end))
            .field("dest_entity", &self.dest_entity_index)
            .field("dest_position", &self.dest_position)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S> SubListChangeMove<S> {
    /// Creates a new sublist change move.
    ///
    /// # Arguments
    /// * `source_entity_index` - Entity index to remove from
    /// * `source_start` - Start of range (inclusive)
    /// * `source_end` - End of range (exclusive)
    /// * `dest_entity_index` - Entity index to insert into
    /// * `dest_position` - Position in destination list
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_entity_index: usize,
        source_start: usize,
        source_end: usize,
        dest_entity_index: usize,
        dest_position: usize,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            source_entity_index,
            source_start,
            source_end,
            dest_entity_index,
            dest_position,
            variable_name,
            descriptor_index,
            indices: [source_entity_index, dest_entity_index],
            _phantom: PhantomData,
        }
    }

    /// Returns the source entity index.
    pub fn source_entity_index(&self) -> usize {
        self.source_entity_index
    }

    /// Returns the source range start (inclusive).
    pub fn source_start(&self) -> usize {
        self.source_start
    }

    /// Returns the source range end (exclusive).
    pub fn source_end(&self) -> usize {
        self.source_end
    }

    /// Returns the sublist length.
    pub fn sublist_len(&self) -> usize {
        self.source_end.saturating_sub(self.source_start)
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

impl<S> Move<S> for SubListChangeMove<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();

        // Check range is valid (start < end)
        if self.source_start >= self.source_end {
            return false;
        }

        // Check source range is within bounds
        let source_len = solution.list_len(self.source_entity_index);
        if self.source_end > source_len {
            return false;
        }

        // Check destination position is valid
        let dest_len = solution.list_len(self.dest_entity_index);
        let sublist_len = self.sublist_len();

        let max_dest = if self.is_intra_list() {
            // After removing sublist, list is shorter
            source_len - sublist_len
        } else {
            dest_len
        };

        if self.dest_position > max_dest {
            return false;
        }

        // For intra-list, check if move would actually change anything
        if self.is_intra_list() {
            // If dest is within the source range, it's a no-op
            if self.dest_position >= self.source_start && self.dest_position <= self.source_end {
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

        // Remove sublist from source
        let elements = score_director
            .working_solution_mut()
            .remove_sublist(self.source_entity_index, self.source_start, self.source_end);

        // dest_position is relative to post-removal list, no adjustment needed
        let dest_pos = self.dest_position;

        // Insert at destination
        score_director
            .working_solution_mut()
            .insert_sublist(self.dest_entity_index, dest_pos, elements.clone());

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

        // Register undo
        let src_entity = self.source_entity_index;
        let src_start = self.source_start;
        let dest_entity = self.dest_entity_index;
        let sublist_len = self.sublist_len();

        score_director.register_undo(Box::new(move |s: &mut S| {
            // Remove from where we inserted
            let removed = s.remove_sublist(dest_entity, dest_pos, dest_pos + sublist_len);
            // Insert back at original position
            s.insert_sublist(src_entity, src_start, removed);
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
    use crate::operations::VariableOperations;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::{RecordingScoreDirector, SimpleScoreDirector};
    use std::any::TypeId;

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
            visits: vec![1, 2, 3, 4, 5, 6],
        }];
        let mut director = create_director(vehicles);

        // Move elements [1..3) (values 2, 3) to end of list
        // After removing [1..3), list is [1, 4, 5, 6], insert at position 4
        let m = SubListChangeMove::<RoutingSolution>::new(0, 1, 3, 0, 4, "visits", 0);

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            // After: [1, 4, 5, 6, 2, 3]
            let visits = &recording.working_solution().vehicles[0].visits;
            assert_eq!(visits, &[1, 4, 5, 6, 2, 3]);

            recording.undo_changes();
        }

        let visits = &director.working_solution().vehicles[0].visits;
        assert_eq!(visits, &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn intra_list_move_backward() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3, 4, 5, 6],
        }];
        let mut director = create_director(vehicles);

        // Move elements [3..5) (values 4, 5) to position 1
        let m = SubListChangeMove::<RoutingSolution>::new(0, 3, 5, 0, 1, "visits", 0);

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            // After: [1, 4, 5, 2, 3, 6]
            let visits = &recording.working_solution().vehicles[0].visits;
            assert_eq!(visits, &[1, 4, 5, 2, 3, 6]);

            recording.undo_changes();
        }

        let visits = &director.working_solution().vehicles[0].visits;
        assert_eq!(visits, &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn inter_list_move() {
        let vehicles = vec![
            Vehicle {
                visits: vec![1, 2, 3, 4],
            },
            Vehicle {
                visits: vec![10, 20],
            },
        ];
        let mut director = create_director(vehicles);

        // Move elements [1..3) (values 2, 3) from vehicle 0 to vehicle 1 at position 1
        let m = SubListChangeMove::<RoutingSolution>::new(0, 1, 3, 1, 1, "visits", 0);

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let sol = recording.working_solution();
            assert_eq!(sol.vehicles[0].visits, vec![1, 4]);
            assert_eq!(sol.vehicles[1].visits, vec![10, 2, 3, 20]);

            recording.undo_changes();
        }

        let sol = director.working_solution();
        assert_eq!(sol.vehicles[0].visits, vec![1, 2, 3, 4]);
        assert_eq!(sol.vehicles[1].visits, vec![10, 20]);
    }

    #[test]
    fn empty_range_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3],
        }];
        let director = create_director(vehicles);

        // start >= end is not doable
        let m = SubListChangeMove::<RoutingSolution>::new(0, 2, 2, 0, 0, "visits", 0);

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn out_of_bounds_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3],
        }];
        let director = create_director(vehicles);

        let m = SubListChangeMove::<RoutingSolution>::new(0, 1, 10, 0, 0, "visits", 0);

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn dest_within_source_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3, 4, 5],
        }];
        let director = create_director(vehicles);

        // Moving [1..4) to position 2 (within the range) is a no-op
        let m = SubListChangeMove::<RoutingSolution>::new(0, 1, 4, 0, 2, "visits", 0);

        assert!(!m.is_doable(&director));
    }
}
