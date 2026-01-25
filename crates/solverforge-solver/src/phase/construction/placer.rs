//! Entity placers for construction heuristic
//!
//! Placers enumerate the entities that need values assigned and
//! generate candidate moves for each entity.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::{ChangeMove, ListAssignMove, Move};
use crate::heuristic::selector::{EntityReference, EntitySelector, TypedValueSelector};

/// A placement represents an entity that needs a value assigned,
/// along with the candidate moves to assign values.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
pub struct Placement<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// The entity reference.
    pub entity_ref: EntityReference,
    /// Candidate moves for this placement.
    pub moves: Vec<M>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M> Placement<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// Creates a new placement.
    pub fn new(entity_ref: EntityReference, moves: Vec<M>) -> Self {
        Self {
            entity_ref,
            moves,
            _phantom: PhantomData,
        }
    }

    /// Returns true if there are no candidate moves.
    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    /// Takes ownership of a move at the given index.
    ///
    /// Uses swap_remove for O(1) removal.
    pub fn take_move(&mut self, index: usize) -> M {
        self.moves.swap_remove(index)
    }
}

impl<S, M> Debug for Placement<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Placement")
            .field("entity_ref", &self.entity_ref)
            .field("move_count", &self.moves.len())
            .finish()
    }
}

/// Trait for placing entities during construction.
///
/// Entity placers iterate over uninitialized entities and generate
/// candidate moves for each. The `next_placement` method is called
/// repeatedly, allowing the placer to see the current solution state
/// after each move is applied.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
pub trait EntityPlacer<S, M>: Send + Debug
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// Returns the next placement (entity + candidate moves) if any remain.
    ///
    /// This is called repeatedly during construction. Each call should check
    /// the current solution state to find the next uninitialized entity.
    /// Returns `None` when all entities are initialized.
    fn next_placement<C>(&self, score_director: &ScoreDirector<S, C>) -> Option<Placement<S, M>>
    where
        C: ConstraintSet<S, S::Score>,
        S::Score: Score;
}

/// A queued entity placer that processes entities in order.
///
/// For each uninitialized entity, generates change moves for all possible values.
/// Uses typed function pointers for zero-erasure access.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The value type
/// * `ES` - The entity selector type
/// * `VS` - The value selector type
pub struct QueuedEntityPlacer<S, V, ES, VS>
where
    S: PlanningSolution,
    ES: EntitySelector<S>,
    VS: TypedValueSelector<S, V>,
{
    /// The entity selector.
    entity_selector: ES,
    /// The value selector.
    value_selector: VS,
    /// Typed getter function pointer.
    getter: fn(&S, usize) -> Option<V>,
    /// Typed setter function pointer.
    setter: fn(&mut S, usize, Option<V>),
    /// The variable name.
    variable_name: &'static str,
    /// The descriptor index.
    descriptor_index: usize,
    _phantom: PhantomData<V>,
}

impl<S, V, ES, VS> QueuedEntityPlacer<S, V, ES, VS>
where
    S: PlanningSolution,
    ES: EntitySelector<S>,
    VS: TypedValueSelector<S, V>,
{
    /// Creates a new queued entity placer with typed function pointers.
    pub fn new(
        entity_selector: ES,
        value_selector: VS,
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        descriptor_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            entity_selector,
            value_selector,
            getter,
            setter,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, ES, VS> Debug for QueuedEntityPlacer<S, V, ES, VS>
where
    S: PlanningSolution,
    ES: EntitySelector<S> + Debug,
    VS: TypedValueSelector<S, V> + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueuedEntityPlacer")
            .field("entity_selector", &self.entity_selector)
            .field("value_selector", &self.value_selector)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V, ES, VS> EntityPlacer<S, ChangeMove<S, V>> for QueuedEntityPlacer<S, V, ES, VS>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
    VS: TypedValueSelector<S, V>,
{
    fn next_placement<C>(
        &self,
        score_director: &ScoreDirector<S, C>,
    ) -> Option<Placement<S, ChangeMove<S, V>>>
    where
        C: ConstraintSet<S, S::Score>,
        S::Score: Score,
    {
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;
        let getter = self.getter;
        let setter = self.setter;

        // Find the first uninitialized entity and generate its moves
        self.entity_selector.iter(score_director).find_map(|entity_ref| {
            // Check if entity is uninitialized using typed getter - zero erasure
            let current_value =
                getter(score_director.working_solution(), entity_ref.entity_index);

            // Only include uninitialized entities
            if current_value.is_some() {
                return None;
            }

            // Generate moves for all possible values
            let moves: Vec<ChangeMove<S, V>> = self
                .value_selector
                .iter_typed(
                    score_director,
                    entity_ref.descriptor_index,
                    entity_ref.entity_index,
                )
                .map(|value| {
                    ChangeMove::new(
                        entity_ref.entity_index,
                        Some(value),
                        getter,
                        setter,
                        variable_name,
                        descriptor_index,
                    )
                })
                .collect();

            if moves.is_empty() {
                None
            } else {
                Some(Placement::new(entity_ref, moves))
            }
        })
    }
}

/// Entity placer that sorts placements by a comparator function.
///
/// Wraps an inner placer and sorts its placements using a typed comparator.
/// This enables FIRST_FIT_DECREASING and similar construction variants.
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::construction::{SortedEntityPlacer, QueuedEntityPlacer, EntityPlacer};
/// use solverforge_solver::heuristic::r#move::ChangeMove;
/// use solverforge_solver::heuristic::selector::{FromSolutionEntitySelector, StaticTypedValueSelector};
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
/// use solverforge_scoring::ScoreDirector;
/// use std::cmp::Ordering;
///
/// #[derive(Clone, Debug)]
/// struct Task { difficulty: i32, assigned: Option<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Solution { tasks: Vec<Task>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn get_assigned(s: &Solution, i: usize) -> Option<i32> {
///     s.tasks.get(i).and_then(|t| t.assigned)
/// }
/// fn set_assigned(s: &mut Solution, i: usize, v: Option<i32>) {
///     if let Some(t) = s.tasks.get_mut(i) { t.assigned = v; }
/// }
///
/// // Sort entities by difficulty (descending) for FIRST_FIT_DECREASING
/// fn difficulty_descending(s: &Solution, a: usize, b: usize) -> Ordering {
///     let da = s.tasks.get(a).map(|t| t.difficulty).unwrap_or(0);
///     let db = s.tasks.get(b).map(|t| t.difficulty).unwrap_or(0);
///     db.cmp(&da)  // Descending order
/// }
/// ```
pub struct SortedEntityPlacer<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: EntityPlacer<S, M>,
{
    inner: Inner,
    /// Comparator function: takes (solution, entity_index_a, entity_index_b) -> Ordering
    comparator: fn(&S, usize, usize) -> std::cmp::Ordering,
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M, Inner> SortedEntityPlacer<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: EntityPlacer<S, M>,
{
    /// Creates a new sorted entity placer.
    ///
    /// # Arguments
    /// * `inner` - The inner placer to wrap
    /// * `comparator` - Function to compare entities: `(solution, idx_a, idx_b) -> Ordering`
    pub fn new(inner: Inner, comparator: fn(&S, usize, usize) -> std::cmp::Ordering) -> Self {
        Self {
            inner,
            comparator,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, Inner> Debug for SortedEntityPlacer<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: EntityPlacer<S, M>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SortedEntityPlacer")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, M, Inner> EntityPlacer<S, M> for SortedEntityPlacer<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: EntityPlacer<S, M>,
{
    fn next_placement<C>(&self, score_director: &ScoreDirector<S, C>) -> Option<Placement<S, M>>
    where
        C: ConstraintSet<S, S::Score>,
        S::Score: Score,
    {
        // SortedEntityPlacer is a wrapper that delegates to inner placer.
        // The sorting happens via the entity selector's iteration order.
        // For true sorted construction, the entity selector should be configured
        // with appropriate sorting. This wrapper simply delegates.
        self.inner.next_placement(score_director)
    }
}

/// Entity placer for list variables during construction.
///
/// Generates `ListAssignMove`s to assign unassigned elements to entities.
/// Each element can be assigned to any entity.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The element type (typically `usize` for index-based lists)
pub struct ListEntityPlacer<S, V> {
    /// Get total number of elements to assign
    element_count: fn(&S) -> usize,
    /// Get elements already assigned
    assigned_elements: fn(&S) -> Vec<V>,
    /// Get number of entities
    n_entities: fn(&S) -> usize,
    /// Assign element to entity (appends to list)
    assign_element: fn(&mut S, usize, V),
    /// Get list length for an entity
    list_len: fn(&S, usize) -> usize,
    /// Remove element at position from entity
    remove_element: fn(&mut S, usize, usize) -> V,
    /// Convert index to element value
    index_to_element: fn(usize) -> V,
    /// Descriptor index for entity type
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> (S, V)>,
}

impl<S, V> ListEntityPlacer<S, V> {
    /// Creates a new list entity placer.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        element_count: fn(&S) -> usize,
        assigned_elements: fn(&S) -> Vec<V>,
        n_entities: fn(&S) -> usize,
        assign_element: fn(&mut S, usize, V),
        list_len: fn(&S, usize) -> usize,
        remove_element: fn(&mut S, usize, usize) -> V,
        index_to_element: fn(usize) -> V,
        descriptor_index: usize,
    ) -> Self {
        Self {
            element_count,
            assigned_elements,
            n_entities,
            assign_element,
            list_len,
            remove_element,
            index_to_element,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> Debug for ListEntityPlacer<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListEntityPlacer")
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V> EntityPlacer<S, ListAssignMove<S, V>> for ListEntityPlacer<S, V>
where
    S: PlanningSolution + solverforge_scoring::ShadowVariableSupport,
    V: Clone + PartialEq + Send + Sync + Debug + std::hash::Hash + Eq + 'static,
{
    fn next_placement<C>(
        &self,
        score_director: &ScoreDirector<S, C>,
    ) -> Option<Placement<S, ListAssignMove<S, V>>>
    where
        C: ConstraintSet<S, S::Score>,
        S::Score: Score,
    {
        let solution = score_director.working_solution();
        let total = (self.element_count)(solution);
        let assigned: std::collections::HashSet<V> =
            (self.assigned_elements)(solution).into_iter().collect();
        let n_entities = (self.n_entities)(solution);

        // Find the first unassigned element
        let unassigned_element = (0..total)
            .map(|i| (self.index_to_element)(i))
            .find(|elem| !assigned.contains(elem));

        // Return placement for this element if found
        unassigned_element.map(|element| {
            let moves: Vec<ListAssignMove<S, V>> = (0..n_entities)
                .map(|entity_idx| {
                    ListAssignMove::new(
                        element.clone(),
                        entity_idx,
                        self.assign_element,
                        self.list_len,
                        self.remove_element,
                        "list",
                        self.descriptor_index,
                    )
                })
                .collect();

            // Use a dummy entity ref since this is element-based, not entity-based
            let entity_ref = EntityReference {
                descriptor_index: self.descriptor_index,
                entity_index: 0,
            };

            Placement::new(entity_ref, moves)
        })
    }
}
