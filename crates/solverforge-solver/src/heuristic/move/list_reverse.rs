//! ListReverseMove - reverses a segment within a list variable.
//!
//! This move reverses the order of elements in a range. Essential for
//! TSP 2-opt optimization where reversing a tour segment can reduce distance.
//!
//! # Zero-Erasure Design
//!
//! Uses typed function pointers for list operations. No `dyn Any`, no downcasting.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Move;

/// A move that reverses a segment within a list.
///
/// This is the fundamental 2-opt move for TSP. Reversing a segment of the tour
/// can significantly reduce total distance by eliminating crossing edges.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `V` - The list element value type
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::ListReverseMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug)]
/// struct Tour { cities: Vec<i32>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Tour {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn list_len(s: &Tour, _: usize) -> usize { s.cities.len() }
/// fn list_reverse(s: &mut Tour, _: usize, start: usize, end: usize) {
///     s.cities[start..end].reverse();
/// }
///
/// // Reverse segment [1..4) in tour: [A, B, C, D, E] -> [A, D, C, B, E]
/// let m = ListReverseMove::<Tour, _, i32>::new(
///     0, 1, 4,
///     list_len, list_reverse,
///     "cities", 0,
/// );
/// ```
pub struct ListReverseMove<S, D, V> {
    /// Entity index
    entity_index: usize,
    /// Start of range to reverse (inclusive)
    start: usize,
    /// End of range to reverse (exclusive)
    end: usize,
    /// Get list length for an entity
    list_len: fn(&S, usize) -> usize,
    /// Reverse elements in range [start, end)
    list_reverse: fn(&mut S, usize, usize, usize),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> D, V)>,
}

// Manual Clone impl to avoid D: Clone bound from derive
impl<S, D, V> Clone for ListReverseMove<S, D, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, D, V> Copy for ListReverseMove<S, D, V> {}

impl<S, D, V: Debug> Debug for ListReverseMove<S, D, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListReverseMove")
            .field("entity", &self.entity_index)
            .field("range", &(self.start..self.end))
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, D, V> ListReverseMove<S, D, V> {
    /// Creates a new list reverse move with typed function pointers.
    ///
    /// # Arguments
    /// * `entity_index` - Entity index
    /// * `start` - Start of range (inclusive)
    /// * `end` - End of range (exclusive)
    /// * `list_len` - Function to get list length
    /// * `list_reverse` - Function to reverse elements in range
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_index: usize,
        start: usize,
        end: usize,
        list_len: fn(&S, usize) -> usize,
        list_reverse: fn(&mut S, usize, usize, usize),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_index,
            start,
            end,
            list_len,
            list_reverse,
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

impl<S, D, V> Move<S, D> for ListReverseMove<S, D, V>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn is_doable(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();

        // Range must have at least 2 elements to be meaningful
        if self.end <= self.start + 1 {
            return false;
        }

        // Check range is within bounds
        let len = (self.list_len)(solution, self.entity_index);
        if self.end > len {
            return false;
        }

        true
    }

    fn do_move(&self, score_director: &mut D) {
        // Notify before change
        score_director.before_variable_changed(
            self.descriptor_index,
            self.entity_index,
            self.variable_name,
        );

        // Reverse the segment
        (self.list_reverse)(
            score_director.working_solution_mut(),
            self.entity_index,
            self.start,
            self.end,
        );

        // Notify after change
        score_director.after_variable_changed(
            self.descriptor_index,
            self.entity_index,
            self.variable_name,
        );

        // Register undo - reversing twice restores original
        let list_reverse = self.list_reverse;
        let entity = self.entity_index;
        let start = self.start;
        let end = self.end;

        score_director.register_undo(Box::new(move |s: &mut S| {
            list_reverse(s, entity, start, end);
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
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::{RecordingScoreDirector, SimpleScoreDirector};
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Tour {
        cities: Vec<i32>,
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

    fn get_tours(s: &TspSolution) -> &Vec<Tour> {
        &s.tours
    }
    fn get_tours_mut(s: &mut TspSolution) -> &mut Vec<Tour> {
        &mut s.tours
    }

    fn list_len(s: &TspSolution, entity_idx: usize) -> usize {
        s.tours.get(entity_idx).map_or(0, |t| t.cities.len())
    }
    fn list_reverse(s: &mut TspSolution, entity_idx: usize, start: usize, end: usize) {
        if let Some(t) = s.tours.get_mut(entity_idx) {
            t.cities[start..end].reverse();
        }
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
        let m =
            ListReverseMove::<TspSolution, _, i32>::new(0, 1, 4, list_len, list_reverse, "cities", 0);

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

        let m =
            ListReverseMove::<TspSolution, _, i32>::new(0, 0, 4, list_len, list_reverse, "cities", 0);

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
        let m =
            ListReverseMove::<TspSolution, _, i32>::new(0, 1, 2, list_len, list_reverse, "cities", 0);

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn out_of_bounds_not_doable() {
        let tours = vec![Tour {
            cities: vec![1, 2, 3],
        }];
        let director = create_director(tours);

        let m =
            ListReverseMove::<TspSolution, _, i32>::new(0, 1, 10, list_len, list_reverse, "cities", 0);

        assert!(!m.is_doable(&director));
    }
}
