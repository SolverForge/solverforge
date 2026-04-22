/* Entity placers for construction heuristic

Placers enumerate the entities that need values assigned and
generate candidate moves for each entity.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{ChangeMove, Move};
use crate::heuristic::selector::{EntityReference, EntitySelector, ValueSelector};

use super::ConstructionSlotId;

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
    // The entity reference.
    pub entity_ref: EntityReference,
    // Candidate moves for this placement.
    pub moves: Vec<M>,
    // Whether keeping the current value is a legal construction choice.
    keep_current_legal: bool,
    slot_id: Option<ConstructionSlotId>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M> Placement<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub fn new(entity_ref: EntityReference, moves: Vec<M>) -> Self {
        Self {
            entity_ref,
            moves,
            keep_current_legal: false,
            slot_id: None,
            _phantom: PhantomData,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    pub fn with_keep_current_legal(mut self, legal: bool) -> Self {
        self.keep_current_legal = legal;
        self
    }

    pub fn keep_current_legal(&self) -> bool {
        self.keep_current_legal
    }

    pub(crate) fn with_slot_id(mut self, slot_id: ConstructionSlotId) -> Self {
        self.slot_id = Some(slot_id);
        self
    }

    pub(crate) fn slot_id(&self) -> Option<ConstructionSlotId> {
        self.slot_id
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
            .field("keep_current_legal", &self.keep_current_legal)
            .field("slot_id", &self.slot_id)
            .finish()
    }
}

/// Trait for placing entities during construction.
///
/// Entity placers iterate over uninitialized entities and generate
/// candidate moves for each.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
pub trait EntityPlacer<S, M>: Send + Debug
where
    S: PlanningSolution,
    M: Move<S>,
{
    // Returns all placements (entities + their candidate moves).
    fn get_placements<D: Director<S>>(&self, score_director: &D) -> Vec<Placement<S, M>>;
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
    VS: ValueSelector<S, V>,
{
    // The entity selector.
    entity_selector: ES,
    // The value selector.
    value_selector: VS,
    // Typed getter function pointer.
    getter: fn(&S, usize) -> Option<V>,
    // Typed setter function pointer.
    setter: fn(&mut S, usize, Option<V>),
    // The variable name.
    variable_name: &'static str,
    // The descriptor index.
    descriptor_index: usize,
    // Whether the variable can remain unassigned during construction.
    allows_unassigned: bool,
    _phantom: PhantomData<fn() -> V>,
}

impl<S, V, ES, VS> QueuedEntityPlacer<S, V, ES, VS>
where
    S: PlanningSolution,
    ES: EntitySelector<S>,
    VS: ValueSelector<S, V>,
{
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
            allows_unassigned: false,
            _phantom: PhantomData,
        }
    }

    pub fn with_allows_unassigned(mut self, allows_unassigned: bool) -> Self {
        self.allows_unassigned = allows_unassigned;
        self
    }
}

impl<S, V, ES, VS> Debug for QueuedEntityPlacer<S, V, ES, VS>
where
    S: PlanningSolution,
    ES: EntitySelector<S> + Debug,
    VS: ValueSelector<S, V> + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueuedEntityPlacer")
            .field("entity_selector", &self.entity_selector)
            .field("value_selector", &self.value_selector)
            .field("variable_name", &self.variable_name)
            .field("allows_unassigned", &self.allows_unassigned)
            .finish()
    }
}

impl<S, V, ES, VS> EntityPlacer<S, ChangeMove<S, V>> for QueuedEntityPlacer<S, V, ES, VS>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
    VS: ValueSelector<S, V>,
{
    fn get_placements<D: Director<S>>(
        &self,
        score_director: &D,
    ) -> Vec<Placement<S, ChangeMove<S, V>>> {
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;
        let getter = self.getter;
        let setter = self.setter;
        let allows_unassigned = self.allows_unassigned;

        self.entity_selector
            .iter(score_director)
            .filter_map(|entity_ref| {
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
                    .iter(
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
                    Some(
                        Placement::new(entity_ref, moves)
                            .with_keep_current_legal(allows_unassigned),
                    )
                }
            })
            .collect()
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
/// use solverforge_solver::heuristic::selector::{FromSolutionEntitySelector, StaticValueSelector};
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
/// use solverforge_scoring::ScoreDirector;
/// use std::cmp::Ordering;
///
/// #[derive(Clone, Debug)]
/// struct Task { difficulty: i32, assigned: Option<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Solution { tasks: Vec<Task>, score: Option<SoftScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SoftScore;
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
    // Comparator function: takes (solution, entity_index_a, entity_index_b) -> Ordering
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
    fn get_placements<D: Director<S>>(&self, score_director: &D) -> Vec<Placement<S, M>> {
        let mut placements = self.inner.get_placements(score_director);
        let solution = score_director.working_solution();
        let cmp = self.comparator;

        placements.sort_by(|a, b| {
            cmp(
                solution,
                a.entity_ref.entity_index,
                b.entity_ref.entity_index,
            )
        });

        placements
    }
}

#[cfg(test)]
#[path = "placer_tests.rs"]
mod tests;
