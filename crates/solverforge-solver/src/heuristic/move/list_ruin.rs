//! ListRuinMove - removes elements from a list variable for LNS.
//!
//! This move removes selected elements from a list, allowing a construction
//! heuristic to reinsert them in potentially better positions.
//!
//! # Zero-Erasure Design
//!
//! Stores only indices. No value type parameter. Operations use VariableOperations trait.

use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::operations::VariableOperations;

use super::Move;

/// A move that removes elements from a list for Large Neighborhood Search.
///
/// Elements are removed by index and stored for reinsertion by a construction
/// heuristic. This is the list-variable equivalent of `RuinMove`.
///
/// # Type Parameters
/// * `S` - The planning solution type (must implement VariableOperations)
#[derive(Clone)]
pub struct ListRuinMove<S> {
    /// Entity index
    entity_index: usize,
    /// Indices of elements to remove (in ascending order for correct removal)
    element_indices: SmallVec<[usize; 8]>,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for ListRuinMove<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListRuinMove")
            .field("entity", &self.entity_index)
            .field("elements", &self.element_indices.as_slice())
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S> ListRuinMove<S> {
    /// Creates a new list ruin move.
    ///
    /// # Arguments
    /// * `entity_index` - Entity index
    /// * `element_indices` - Indices of elements to remove
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    ///
    /// # Note
    /// Indices are sorted internally for correct removal order.
    pub fn new(
        entity_index: usize,
        element_indices: &[usize],
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        let mut indices: SmallVec<[usize; 8]> = SmallVec::from_slice(element_indices);
        indices.sort_unstable();
        Self {
            entity_index,
            element_indices: indices,
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

impl<S> Move<S> for ListRuinMove<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        if self.element_indices.is_empty() {
            return false;
        }

        let solution = score_director.working_solution();
        let len = solution.list_len(self.entity_index);

        // All indices must be within bounds
        self.element_indices.iter().all(|&idx| idx < len)
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        let entity = self.entity_index;
        let descriptor = self.descriptor_index;
        let variable_name = self.variable_name;

        // Notify before change
        score_director.before_variable_changed(descriptor, entity, variable_name);

        // Remove elements in reverse order (highest index first) to preserve indices
        let mut removed: SmallVec<[(usize, <S as VariableOperations>::Element); 8]> =
            SmallVec::new();
        for &idx in self.element_indices.iter().rev() {
            let value = score_director.working_solution_mut().remove(entity, idx);
            removed.push((idx, value));
        }

        // Notify after change
        score_director.after_variable_changed(descriptor, entity, variable_name);

        // Register undo - reinsert in original order (lowest index first)
        score_director.register_undo(Box::new(move |s: &mut S| {
            for (idx, value) in removed.into_iter().rev() {
                s.insert(entity, idx, value);
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
    struct Route {
        stops: Vec<usize>,
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

    impl VariableOperations for VrpSolution {
        type Element = usize;

        fn element_count(&self) -> usize {
            self.routes.iter().map(|r| r.stops.len()).sum()
        }

        fn entity_count(&self) -> usize {
            self.routes.len()
        }

        fn assigned_elements(&self) -> Vec<Self::Element> {
            self.routes
                .iter()
                .flat_map(|r| r.stops.iter().copied())
                .collect()
        }

        fn assign(&mut self, entity_idx: usize, elem: Self::Element) {
            self.routes[entity_idx].stops.push(elem);
        }

        fn list_len(&self, entity_idx: usize) -> usize {
            self.routes.get(entity_idx).map_or(0, |r| r.stops.len())
        }

        fn remove(&mut self, entity_idx: usize, pos: usize) -> Self::Element {
            self.routes[entity_idx].stops.remove(pos)
        }

        fn insert(&mut self, entity_idx: usize, pos: usize, elem: Self::Element) {
            self.routes[entity_idx].stops.insert(pos, elem);
        }

        fn get(&self, entity_idx: usize, pos: usize) -> Self::Element {
            self.routes[entity_idx].stops[pos]
        }

        fn descriptor_index() -> usize {
            0
        }

        fn variable_name() -> &'static str {
            "stops"
        }

        fn is_list_variable() -> bool {
            true
        }
    }

    fn get_routes(s: &VrpSolution) -> &Vec<Route> {
        &s.routes
    }
    fn get_routes_mut(s: &mut VrpSolution) -> &mut Vec<Route> {
        &mut s.routes
    }

    fn create_director(
        stops: Vec<usize>,
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

        let m = ListRuinMove::<VrpSolution>::new(0, &[2], "stops", 0);

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
        let m = ListRuinMove::<VrpSolution>::new(0, &[1, 3], "stops", 0);

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
        let m = ListRuinMove::<VrpSolution>::new(0, &[3, 1], "stops", 0);

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

        let m = ListRuinMove::<VrpSolution>::new(0, &[], "stops", 0);

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn out_of_bounds_not_doable() {
        let director = create_director(vec![1, 2, 3]);

        let m = ListRuinMove::<VrpSolution>::new(0, &[0, 10], "stops", 0);

        assert!(!m.is_doable(&director));
    }
}
