//! ListReverseMove - reverses a segment within a list variable.
//!
//! This move reverses the order of elements in a range. Essential for
//! TSP 2-opt optimization where reversing a tour segment can reduce distance.
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

/// A move that reverses a segment within a list.
///
/// This is the fundamental 2-opt move for TSP. Reversing a segment of the tour
/// can significantly reduce total distance by eliminating crossing edges.
///
/// # Type Parameters
/// * `S` - The planning solution type (must implement VariableOperations)
#[derive(Clone, Copy)]
pub struct ListReverseMove<S> {
    /// Entity index
    entity_index: usize,
    /// Start of range to reverse (inclusive)
    start: usize,
    /// End of range to reverse (exclusive)
    end: usize,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for ListReverseMove<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListReverseMove")
            .field("entity", &self.entity_index)
            .field("range", &(self.start..self.end))
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S> ListReverseMove<S> {
    /// Creates a new list reverse move.
    ///
    /// # Arguments
    /// * `entity_index` - Entity index
    /// * `start` - Start of range (inclusive)
    /// * `end` - End of range (exclusive)
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    pub fn new(
        entity_index: usize,
        start: usize,
        end: usize,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_index,
            start,
            end,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    /// Returns the entity index.
    pub fn entity_index(&self) -> usize {
        self.entity_index
    }

    /// Returns the range start.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the range end.
    pub fn end(&self) -> usize {
        self.end
    }

    /// Returns the segment length.
    pub fn segment_len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

impl<S> Move<S> for ListReverseMove<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();

        // Range must have at least 2 elements to be meaningful
        if self.end <= self.start + 1 {
            return false;
        }

        // Check range is within bounds
        let len = solution.list_len(self.entity_index);
        if self.end > len {
            return false;
        }

        true
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        // Notify before change
        score_director.before_variable_changed(
            self.descriptor_index,
            self.entity_index,
            self.variable_name,
        );

        // Collect elements to reverse
        let mut elements = Vec::with_capacity(self.end - self.start);
        {
            let sol = score_director.working_solution();
            for i in self.start..self.end {
                elements.push(sol.get(self.entity_index, i));
            }
        }

        // Remove and reinsert in reverse order
        {
            let sol = score_director.working_solution_mut();
            for i in (self.start..self.end).rev() {
                sol.remove(self.entity_index, i);
            }
            for (i, elem) in elements.iter().rev().enumerate() {
                sol.insert(self.entity_index, self.start + i, *elem);
            }
        }

        // Notify after change
        score_director.after_variable_changed(
            self.descriptor_index,
            self.entity_index,
            self.variable_name,
        );

        // Register undo - reversing twice restores original
        let entity = self.entity_index;
        let start = self.start;
        let end = self.end;

        score_director.register_undo(Box::new(move |s: &mut S| {
            // Collect current elements
            let mut elems = Vec::with_capacity(end - start);
            for i in start..end {
                elems.push(s.get(entity, i));
            }
            // Remove and reinsert reversed
            for i in (start..end).rev() {
                s.remove(entity, i);
            }
            for (i, elem) in elems.iter().rev().enumerate() {
                s.insert(entity, start + i, *elem);
            }
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        std::slice::from_ref(&self.entity_index)
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
    struct Tour {
        cities: Vec<usize>,
    }

    #[derive(Clone, Debug)]
    struct TspSolution {
        tours: Vec<Tour>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TspSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    impl VariableOperations for TspSolution {
        type Element = usize;

        fn element_count(&self) -> usize {
            self.tours.iter().map(|t| t.cities.len()).sum()
        }

        fn entity_count(&self) -> usize {
            self.tours.len()
        }

        fn assigned_elements(&self) -> Vec<Self::Element> {
            self.tours
                .iter()
                .flat_map(|t| t.cities.iter().copied())
                .collect()
        }

        fn assign(&mut self, entity_idx: usize, elem: Self::Element) {
            self.tours[entity_idx].cities.push(elem);
        }

        fn list_len(&self, entity_idx: usize) -> usize {
            self.tours.get(entity_idx).map_or(0, |t| t.cities.len())
        }

        fn remove(&mut self, entity_idx: usize, pos: usize) -> Self::Element {
            self.tours[entity_idx].cities.remove(pos)
        }

        fn insert(&mut self, entity_idx: usize, pos: usize, elem: Self::Element) {
            self.tours[entity_idx].cities.insert(pos, elem);
        }

        fn get(&self, entity_idx: usize, pos: usize) -> Self::Element {
            self.tours[entity_idx].cities[pos]
        }

        fn descriptor_index() -> usize {
            0
        }

        fn variable_name() -> &'static str {
            "cities"
        }

        fn is_list_variable() -> bool {
            true
        }
    }

    fn get_tours(s: &TspSolution) -> &Vec<Tour> {
        &s.tours
    }
    fn get_tours_mut(s: &mut TspSolution) -> &mut Vec<Tour> {
        &mut s.tours
    }

    fn create_director(
        tours: Vec<Tour>,
    ) -> SimpleScoreDirector<TspSolution, impl Fn(&TspSolution) -> SimpleScore> {
        let solution = TspSolution { tours, score: None };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Tour",
            "tours",
            get_tours,
            get_tours_mut,
        ));
        let entity_desc =
            EntityDescriptor::new("Tour", TypeId::of::<Tour>(), "tours").with_extractor(extractor);
        let descriptor = SolutionDescriptor::new("TspSolution", TypeId::of::<TspSolution>())
            .with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn reverse_segment() {
        let tours = vec![Tour {
            cities: vec![1, 2, 3, 4, 5],
        }];
        let mut director = create_director(tours);

        // Reverse [1..4): [1, 2, 3, 4, 5] -> [1, 4, 3, 2, 5]
        let m = ListReverseMove::<TspSolution>::new(0, 1, 4, "cities", 0);

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let cities = &recording.working_solution().tours[0].cities;
            assert_eq!(cities, &[1, 4, 3, 2, 5]);

            recording.undo_changes();
        }

        let cities = &director.working_solution().tours[0].cities;
        assert_eq!(cities, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn reverse_entire_list() {
        let tours = vec![Tour {
            cities: vec![1, 2, 3, 4],
        }];
        let mut director = create_director(tours);

        let m = ListReverseMove::<TspSolution>::new(0, 0, 4, "cities", 0);

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let cities = &recording.working_solution().tours[0].cities;
            assert_eq!(cities, &[4, 3, 2, 1]);

            recording.undo_changes();
        }

        let cities = &director.working_solution().tours[0].cities;
        assert_eq!(cities, &[1, 2, 3, 4]);
    }

    #[test]
    fn single_element_not_doable() {
        let tours = vec![Tour {
            cities: vec![1, 2, 3],
        }];
        let director = create_director(tours);

        // Reversing a single element is a no-op
        let m = ListReverseMove::<TspSolution>::new(0, 1, 2, "cities", 0);

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn out_of_bounds_not_doable() {
        let tours = vec![Tour {
            cities: vec![1, 2, 3],
        }];
        let director = create_director(tours);

        let m = ListReverseMove::<TspSolution>::new(0, 1, 10, "cities", 0);

        assert!(!m.is_doable(&director));
    }
}
