//! SubListSwapMove - swaps two contiguous sublists within or between list variables.
//!
//! This move exchanges two ranges of elements. Essential for vehicle routing
//! where segments need to be swapped between vehicles.
//!
//! # Zero-Erasure Design
//!
//! Uses typed function pointers for list operations. No `dyn Any`, no downcasting.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Move;

/// A move that swaps two contiguous sublists.
///
/// Supports both intra-list swaps (within same entity) and inter-list swaps
/// (between different entities). Uses typed function pointers for zero-erasure.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The list element value type
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::SubListSwapMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug)]
/// struct Vehicle { id: usize, visits: Vec<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Solution { vehicles: Vec<Vehicle>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn list_len(s: &Solution, entity_idx: usize) -> usize {
///     s.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
/// }
/// fn sublist_remove(s: &mut Solution, entity_idx: usize, start: usize, end: usize) -> Vec<i32> {
///     s.vehicles.get_mut(entity_idx)
///         .map(|v| v.visits.drain(start..end).collect())
///         .unwrap_or_default()
/// }
/// fn sublist_insert(s: &mut Solution, entity_idx: usize, pos: usize, items: Vec<i32>) {
///     if let Some(v) = s.vehicles.get_mut(entity_idx) {
///         for (i, item) in items.into_iter().enumerate() {
///             v.visits.insert(pos + i, item);
///         }
///     }
/// }
///
/// // Swap [1..3) from vehicle 0 with [0..2) from vehicle 1
/// let m = SubListSwapMove::<Solution, i32>::new(
///     0, 1, 3,  // first: entity 0, range [1, 3)
///     1, 0, 2,  // second: entity 1, range [0, 2)
///     list_len, sublist_remove, sublist_insert,
///     "visits", 0,
/// );
/// ```
pub struct SubListSwapMove<S, V> {
    /// First entity index
    first_entity_index: usize,
    /// Start of first range (inclusive)
    first_start: usize,
    /// End of first range (exclusive)
    first_end: usize,
    /// Second entity index
    second_entity_index: usize,
    /// Start of second range (inclusive)
    second_start: usize,
    /// End of second range (exclusive)
    second_end: usize,
    /// Get list length for an entity
    list_len: fn(&S, usize) -> usize,
    /// Remove sublist [start, end), returns removed elements
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    /// Insert elements at position
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    /// Store indices for entity_indices()
    indices: [usize; 2],
    _phantom: PhantomData<V>,
}

impl<S, V> Clone for SubListSwapMove<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> Copy for SubListSwapMove<S, V> {}

impl<S, V: Debug> Debug for SubListSwapMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubListSwapMove")
            .field("first_entity", &self.first_entity_index)
            .field("first_range", &(self.first_start..self.first_end))
            .field("second_entity", &self.second_entity_index)
            .field("second_range", &(self.second_start..self.second_end))
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> SubListSwapMove<S, V> {
    /// Creates a new sublist swap move with typed function pointers.
    ///
    /// # Arguments
    /// * `first_entity_index` - First entity index
    /// * `first_start` - Start of first range (inclusive)
    /// * `first_end` - End of first range (exclusive)
    /// * `second_entity_index` - Second entity index
    /// * `second_start` - Start of second range (inclusive)
    /// * `second_end` - End of second range (exclusive)
    /// * `list_len` - Function to get list length
    /// * `sublist_remove` - Function to remove range [start, end)
    /// * `sublist_insert` - Function to insert elements at position
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        first_entity_index: usize,
        first_start: usize,
        first_end: usize,
        second_entity_index: usize,
        second_start: usize,
        second_end: usize,
        list_len: fn(&S, usize) -> usize,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            first_entity_index,
            first_start,
            first_end,
            second_entity_index,
            second_start,
            second_end,
            list_len,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            indices: [first_entity_index, second_entity_index],
            _phantom: PhantomData,
        }
    }

    /// Returns the first entity index.
    pub fn first_entity_index(&self) -> usize {
        self.first_entity_index
    }

    /// Returns the first range start.
    pub fn first_start(&self) -> usize {
        self.first_start
    }

    /// Returns the first range end.
    pub fn first_end(&self) -> usize {
        self.first_end
    }

    /// Returns the first sublist length.
    pub fn first_len(&self) -> usize {
        self.first_end.saturating_sub(self.first_start)
    }

    /// Returns the second entity index.
    pub fn second_entity_index(&self) -> usize {
        self.second_entity_index
    }

    /// Returns the second range start.
    pub fn second_start(&self) -> usize {
        self.second_start
    }

    /// Returns the second range end.
    pub fn second_end(&self) -> usize {
        self.second_end
    }

    /// Returns the second sublist length.
    pub fn second_len(&self) -> usize {
        self.second_end.saturating_sub(self.second_start)
    }

    /// Returns true if this is an intra-list swap (same entity).
    pub fn is_intra_list(&self) -> bool {
        self.first_entity_index == self.second_entity_index
    }
}

impl<S, V> Move<S> for SubListSwapMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();

        // Both ranges must be valid (start < end)
        if self.first_start >= self.first_end || self.second_start >= self.second_end {
            return false;
        }

        // Check first range is within bounds
        let first_list_len = (self.list_len)(solution, self.first_entity_index);
        if self.first_end > first_list_len {
            return false;
        }

        // Check second range is within bounds
        let second_list_len = (self.list_len)(solution, self.second_entity_index);
        if self.second_end > second_list_len {
            return false;
        }

        // For intra-list swaps, ranges must not overlap
        if self.is_intra_list() {
            // Ranges overlap if one starts before the other ends
            let overlaps = self.first_start < self.second_end && self.second_start < self.first_end;
            if overlaps {
                return false;
            }
        }

        true
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        // Notify before changes
        score_director.before_variable_changed(
            self.descriptor_index,
            self.first_entity_index,
            self.variable_name,
        );
        if !self.is_intra_list() {
            score_director.before_variable_changed(
                self.descriptor_index,
                self.second_entity_index,
                self.variable_name,
            );
        }

        if self.is_intra_list() {
            // Intra-list swap: need to handle carefully due to index shifts
            // Always process the later range first to avoid index shifts affecting the earlier range
            let (early_start, early_end, late_start, late_end) =
                if self.first_start < self.second_start {
                    (
                        self.first_start,
                        self.first_end,
                        self.second_start,
                        self.second_end,
                    )
                } else {
                    (
                        self.second_start,
                        self.second_end,
                        self.first_start,
                        self.first_end,
                    )
                };

            // Remove later range first
            let late_elements = (self.sublist_remove)(
                score_director.working_solution_mut(),
                self.first_entity_index,
                late_start,
                late_end,
            );

            // Remove earlier range
            let early_elements = (self.sublist_remove)(
                score_director.working_solution_mut(),
                self.first_entity_index,
                early_start,
                early_end,
            );

            // Insert late elements at early position
            (self.sublist_insert)(
                score_director.working_solution_mut(),
                self.first_entity_index,
                early_start,
                late_elements.clone(),
            );

            // Insert early elements at adjusted late position
            // After removing early range, late_start shifts by early_len
            // After inserting late elements, it shifts back by late_len
            let late_len = late_end - late_start;
            let early_len = early_end - early_start;
            let new_late_pos = late_start - early_len + late_len;
            (self.sublist_insert)(
                score_director.working_solution_mut(),
                self.first_entity_index,
                new_late_pos,
                early_elements.clone(),
            );

            // Register undo - swap back
            let sublist_remove = self.sublist_remove;
            let sublist_insert = self.sublist_insert;
            let entity = self.first_entity_index;

            score_director.register_undo(Box::new(move |s: &mut S| {
                // Remove late elements (now at early position with late_len)
                let late_at_early = sublist_remove(s, entity, early_start, early_start + late_len);
                // Remove early elements (now at new_late_pos with early_len)
                let early_at_late = sublist_remove(
                    s,
                    entity,
                    new_late_pos - late_len,
                    new_late_pos - late_len + early_len,
                );
                // Insert early back at early
                sublist_insert(s, entity, early_start, early_at_late);
                // Insert late back at late
                sublist_insert(s, entity, late_start, late_at_early);
            }));
        } else {
            // Inter-list swap: simpler, no index interaction between lists
            let first_elements = (self.sublist_remove)(
                score_director.working_solution_mut(),
                self.first_entity_index,
                self.first_start,
                self.first_end,
            );

            let second_elements = (self.sublist_remove)(
                score_director.working_solution_mut(),
                self.second_entity_index,
                self.second_start,
                self.second_end,
            );

            // Insert swapped
            (self.sublist_insert)(
                score_director.working_solution_mut(),
                self.first_entity_index,
                self.first_start,
                second_elements.clone(),
            );

            (self.sublist_insert)(
                score_director.working_solution_mut(),
                self.second_entity_index,
                self.second_start,
                first_elements.clone(),
            );

            // Register undo
            let sublist_remove = self.sublist_remove;
            let sublist_insert = self.sublist_insert;
            let first_entity = self.first_entity_index;
            let first_start = self.first_start;
            let second_entity = self.second_entity_index;
            let second_start = self.second_start;
            let first_len = self.first_len();
            let second_len = self.second_len();

            score_director.register_undo(Box::new(move |s: &mut S| {
                // Remove second elements from first list
                let second_at_first =
                    sublist_remove(s, first_entity, first_start, first_start + second_len);
                // Remove first elements from second list
                let first_at_second =
                    sublist_remove(s, second_entity, second_start, second_start + first_len);
                // Restore originals
                sublist_insert(s, first_entity, first_start, first_at_second);
                sublist_insert(s, second_entity, second_start, second_at_first);
            }));
        }

        // Notify after changes
        score_director.after_variable_changed(
            self.descriptor_index,
            self.first_entity_index,
            self.variable_name,
        );
        if !self.is_intra_list() {
            score_director.after_variable_changed(
                self.descriptor_index,
                self.second_entity_index,
                self.variable_name,
            );
        }
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
#[path = "sublist_swap_tests.rs"]
mod tests;
