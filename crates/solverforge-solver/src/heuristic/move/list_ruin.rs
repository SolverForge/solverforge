//! ListRuinMove - removes elements from a list variable for LNS.
//!
//! This move removes selected elements from a list, allowing a construction
//! heuristic to reinsert them in potentially better positions.
//!
//! # Zero-Erasure Design
//!
//! Uses typed function pointers for list operations. No `dyn Any`, no downcasting.

use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Move;

/// A move that removes elements from a list for Large Neighborhood Search.
///
/// Elements are removed by index and stored for reinsertion by a construction
/// heuristic. This is the list-variable equivalent of `RuinMove`.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The list element value type
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::ListRuinMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug)]
/// struct Route { stops: Vec<i32>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Route {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn list_len(s: &Route, _: usize) -> usize { s.stops.len() }
/// fn list_remove(s: &mut Route, _: usize, idx: usize) -> i32 { s.stops.remove(idx) }
/// fn list_insert(s: &mut Route, _: usize, idx: usize, v: i32) { s.stops.insert(idx, v); }
///
/// // Remove elements at indices 1 and 3 from the route
/// let m = ListRuinMove::<Route, i32>::new(
///     0,
///     &[1, 3],
///     list_len, list_remove, list_insert,
///     "stops", 0,
/// );
/// ```
#[derive(Clone)]
pub struct ListRuinMove<S, V> {
    /// Entity index
    entity_index: usize,
    /// Indices of elements to remove (in ascending order for correct removal)
    element_indices: SmallVec<[usize; 8]>,
    /// Get list length
    list_len: fn(&S, usize) -> usize,
    /// Remove element at index, returning it
    list_remove: fn(&mut S, usize, usize) -> V,
    /// Insert element at index
    list_insert: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<V>,
}

impl<S, V: Debug> Debug for ListRuinMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListRuinMove")
            .field("entity", &self.entity_index)
            .field("elements", &self.element_indices.as_slice())
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> ListRuinMove<S, V> {
    /// Creates a new list ruin move with typed function pointers.
    ///
    /// # Arguments
    /// * `entity_index` - Entity index
    /// * `element_indices` - Indices of elements to remove
    /// * `list_len` - Function to get list length
    /// * `list_remove` - Function to remove element at index
    /// * `list_insert` - Function to insert element at index
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    ///
    /// # Note
    /// Indices are sorted internally for correct removal order.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_index: usize,
        element_indices: &[usize],
        list_len: fn(&S, usize) -> usize,
        list_remove: fn(&mut S, usize, usize) -> V,
        list_insert: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        let mut indices: SmallVec<[usize; 8]> = SmallVec::from_slice(element_indices);
        indices.sort_unstable();
        Self {
            entity_index,
            element_indices: indices,
            list_len,
            list_remove,
            list_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    /// Returns the entity index.
    pub fn entity_index(&self) -> usize {
        self.entity_index
    }

    /// Returns the element indices being removed.
    pub fn element_indices(&self) -> &[usize] {
        &self.element_indices
    }

    /// Returns the number of elements being removed.
    pub fn ruin_count(&self) -> usize {
        self.element_indices.len()
    }
}

impl<S, V> Move<S> for ListRuinMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn is_doable(&self, score_director: &dyn ScoreDirector<S>) -> bool {
        if self.element_indices.is_empty() {
            return false;
        }

        let solution = score_director.working_solution();
        let len = (self.list_len)(solution, self.entity_index);

        // All indices must be within bounds
        self.element_indices.iter().all(|&idx| idx < len)
    }

    fn do_move(&self, score_director: &mut dyn ScoreDirector<S>) {
        let list_remove = self.list_remove;
        let list_insert = self.list_insert;
        let entity = self.entity_index;
        let descriptor = self.descriptor_index;
        let variable_name = self.variable_name;

        // Notify before change
        score_director.before_variable_changed(descriptor, entity, variable_name);

        // Remove elements in reverse order (highest index first) to preserve indices
        let mut removed: SmallVec<[(usize, V); 8]> = SmallVec::new();
        for &idx in self.element_indices.iter().rev() {
            let value = list_remove(score_director.working_solution_mut(), entity, idx);
            removed.push((idx, value));
        }

        // Notify after change
        score_director.after_variable_changed(descriptor, entity, variable_name);

        // Register undo - reinsert in original order (lowest index first)
        score_director.register_undo(Box::new(move |s: &mut S| {
            for (idx, value) in removed.into_iter().rev() {
                list_insert(s, entity, idx, value);
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
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::{RecordingScoreDirector, SimpleScoreDirector};
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Route {
        stops: Vec<i32>,
    }

    #[derive(Clone, Debug)]
    struct VrpSolution {
        routes: Vec<Route>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for VrpSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_routes(s: &VrpSolution) -> &Vec<Route> {
        &s.routes
    }
    fn get_routes_mut(s: &mut VrpSolution) -> &mut Vec<Route> {
        &mut s.routes
    }

    fn list_len(s: &VrpSolution, entity_idx: usize) -> usize {
        s.routes.get(entity_idx).map_or(0, |r| r.stops.len())
    }
    fn list_remove(s: &mut VrpSolution, entity_idx: usize, idx: usize) -> i32 {
        s.routes
            .get_mut(entity_idx)
            .map(|r| r.stops.remove(idx))
            .unwrap_or(0)
    }
    fn list_insert(s: &mut VrpSolution, entity_idx: usize, idx: usize, v: i32) {
        if let Some(r) = s.routes.get_mut(entity_idx) {
            r.stops.insert(idx, v);
        }
    }

    fn create_director(
        stops: Vec<i32>,
    ) -> SimpleScoreDirector<VrpSolution, impl Fn(&VrpSolution) -> SimpleScore> {
        let routes = vec![Route { stops }];
        let solution = VrpSolution {
            routes,
            score: None,
        };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Route",
            "routes",
            get_routes,
            get_routes_mut,
        ));
        let entity_desc = EntityDescriptor::new("Route", TypeId::of::<Route>(), "routes")
            .with_extractor(extractor);
        let descriptor = SolutionDescriptor::new("VrpSolution", TypeId::of::<VrpSolution>())
            .with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn ruin_single_element() {
        let mut director = create_director(vec![1, 2, 3, 4, 5]);

        let m = ListRuinMove::<VrpSolution, i32>::new(
            0,
            &[2],
            list_len,
            list_remove,
            list_insert,
            "stops",
            0,
        );

        assert!(m.is_doable(&director));
        assert_eq!(m.ruin_count(), 1);

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let stops = &recording.working_solution().routes[0].stops;
            assert_eq!(stops, &[1, 2, 4, 5]);

            recording.undo_changes();
        }

        let stops = &director.working_solution().routes[0].stops;
        assert_eq!(stops, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn ruin_multiple_elements() {
        let mut director = create_director(vec![1, 2, 3, 4, 5]);

        // Remove indices 1, 3 (values 2, 4)
        let m = ListRuinMove::<VrpSolution, i32>::new(
            0,
            &[1, 3],
            list_len,
            list_remove,
            list_insert,
            "stops",
            0,
        );

        assert!(m.is_doable(&director));
        assert_eq!(m.ruin_count(), 2);

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let stops = &recording.working_solution().routes[0].stops;
            assert_eq!(stops, &[1, 3, 5]);

            recording.undo_changes();
        }

        let stops = &director.working_solution().routes[0].stops;
        assert_eq!(stops, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn ruin_unordered_indices() {
        let mut director = create_director(vec![1, 2, 3, 4, 5]);

        // Indices provided in reverse order - should still work
        let m = ListRuinMove::<VrpSolution, i32>::new(
            0,
            &[3, 1],
            list_len,
            list_remove,
            list_insert,
            "stops",
            0,
        );

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let stops = &recording.working_solution().routes[0].stops;
            assert_eq!(stops, &[1, 3, 5]);

            recording.undo_changes();
        }

        let stops = &director.working_solution().routes[0].stops;
        assert_eq!(stops, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn empty_indices_not_doable() {
        let director = create_director(vec![1, 2, 3]);

        let m = ListRuinMove::<VrpSolution, i32>::new(
            0,
            &[],
            list_len,
            list_remove,
            list_insert,
            "stops",
            0,
        );

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn out_of_bounds_not_doable() {
        let director = create_director(vec![1, 2, 3]);

        let m = ListRuinMove::<VrpSolution, i32>::new(
            0,
            &[0, 10],
            list_len,
            list_remove,
            list_insert,
            "stops",
            0,
        );

        assert!(!m.is_doable(&director));
    }
}
