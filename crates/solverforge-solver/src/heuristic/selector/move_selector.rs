/* Typed move selectors for zero-allocation move generation.

Typed move selectors yield concrete move types directly, enabling
monomorphization and arena allocation.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::MoveArena;
use crate::heuristic::r#move::{ChangeMove, Move, SwapMove};

use super::entity::{EntityReference, EntitySelector, FromSolutionEntitySelector};
use super::value_selector::{StaticValueSelector, ValueSelector};

mod either;

pub use either::{ScalarChangeMoveSelector, ScalarSwapMoveSelector};

/// A typed move selector that yields moves of type `M` directly.
///
/// Unlike erased selectors, this returns concrete moves inline,
/// eliminating heap allocation per move.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
pub trait MoveSelector<S: PlanningSolution, M: Move<S>>: Send + Debug {
    // Opens an owned move cursor that must not borrow the score director.
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = M> + 'a;

    // Returns an iterator over typed moves.
    fn iter_moves<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = M> + 'a {
        self.open_cursor(score_director)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize;

    fn append_moves<D: Director<S>>(&self, score_director: &D, arena: &mut MoveArena<M>) {
        arena.extend(self.open_cursor(score_director));
    }

    // Returns true if this selector may return the same move multiple times.
    fn is_never_ending(&self) -> bool {
        false
    }
}

/// A change move selector that generates `ChangeMove` instances.
///
/// Stores typed function pointers for zero-erasure move generation.
pub struct ChangeMoveSelector<S, V, ES, VS> {
    entity_selector: ES,
    value_selector: VS,
    getter: fn(&S, usize) -> Option<V>,
    setter: fn(&mut S, usize, Option<V>),
    descriptor_index: usize,
    variable_name: &'static str,
    allows_unassigned: bool,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V: Debug, ES: Debug, VS: Debug> Debug for ChangeMoveSelector<S, V, ES, VS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("value_selector", &self.value_selector)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .field("allows_unassigned", &self.allows_unassigned)
            .finish()
    }
}

impl<S: PlanningSolution, V: Clone, ES, VS> ChangeMoveSelector<S, V, ES, VS> {
    /// Creates a new change move selector with typed function pointers.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to modify
    /// * `value_selector` - Selects values to assign
    /// * `getter` - Function pointer to get current value from solution
    /// * `setter` - Function pointer to set value on solution
    /// * `descriptor_index` - Index of the entity descriptor
    /// * `variable_name` - Name of the variable
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
            descriptor_index,
            variable_name,
            allows_unassigned: false,
            _phantom: PhantomData,
        }
    }

    pub fn with_allows_unassigned(mut self, allows_unassigned: bool) -> Self {
        self.allows_unassigned = allows_unassigned;
        self
    }
}

impl<S: PlanningSolution, V: Clone + Send + Sync + Debug + 'static>
    ChangeMoveSelector<S, V, FromSolutionEntitySelector, StaticValueSelector<S, V>>
{
    pub fn simple(
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        descriptor_index: usize,
        variable_name: &'static str,
        values: Vec<V>,
    ) -> Self {
        Self {
            entity_selector: FromSolutionEntitySelector::new(descriptor_index),
            value_selector: StaticValueSelector::new(values),
            getter,
            setter,
            descriptor_index,
            variable_name,
            allows_unassigned: false,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, ES, VS> MoveSelector<S, ChangeMove<S, V>> for ChangeMoveSelector<S, V, ES, VS>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
    VS: ValueSelector<S, V>,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = ChangeMove<S, V>> + 'a {
        let descriptor_index = self.descriptor_index;
        let variable_name = self.variable_name;
        let getter = self.getter;
        let setter = self.setter;
        let allows_unassigned = self.allows_unassigned;
        let value_selector = &self.value_selector;
        let solution = score_director.working_solution();
        let entity_values: Vec<_> = self
            .entity_selector
            .iter(score_director)
            .map(|entity_ref| {
                let current_assigned = getter(solution, entity_ref.entity_index).is_some();
                let values = value_selector.iter(
                    score_director,
                    entity_ref.descriptor_index,
                    entity_ref.entity_index,
                );
                (entity_ref, values, current_assigned)
            })
            .collect();

        entity_values
            .into_iter()
            .flat_map(move |(entity_ref, values, current_assigned)| {
                let to_none = (allows_unassigned && current_assigned).then(|| {
                    ChangeMove::new(
                        entity_ref.entity_index,
                        None,
                        getter,
                        setter,
                        variable_name,
                        descriptor_index,
                    )
                });
                values
                    .map(move |value| {
                        ChangeMove::new(
                            entity_ref.entity_index,
                            Some(value),
                            getter,
                            setter,
                            variable_name,
                            descriptor_index,
                        )
                    })
                    .chain(to_none)
            })
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.entity_selector
            .iter(score_director)
            .map(|entity_ref| {
                self.value_selector.size(
                    score_director,
                    entity_ref.descriptor_index,
                    entity_ref.entity_index,
                ) + usize::from(
                    self.allows_unassigned
                        && (self.getter)(
                            score_director.working_solution(),
                            entity_ref.entity_index,
                        )
                        .is_some(),
                )
            })
            .sum()
    }
}

/// A swap move selector that generates `SwapMove` instances.
///
/// Uses typed function pointers for zero-erasure access to variable values.
pub struct SwapMoveSelector<S, V, LES, RES> {
    left_entity_selector: LES,
    right_entity_selector: RES,
    // Typed getter function pointer - zero erasure.
    getter: fn(&S, usize) -> Option<V>,
    // Typed setter function pointer - zero erasure.
    setter: fn(&mut S, usize, Option<V>),
    descriptor_index: usize,
    variable_name: &'static str,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V, LES: Debug, RES: Debug> Debug for SwapMoveSelector<S, V, LES, RES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwapMoveSelector")
            .field("left_entity_selector", &self.left_entity_selector)
            .field("right_entity_selector", &self.right_entity_selector)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S: PlanningSolution, V, LES, RES> SwapMoveSelector<S, V, LES, RES> {
    pub fn new(
        left_entity_selector: LES,
        right_entity_selector: RES,
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        descriptor_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            left_entity_selector,
            right_entity_selector,
            getter,
            setter,
            descriptor_index,
            variable_name,
            _phantom: PhantomData,
        }
    }
}

impl<S: PlanningSolution, V>
    SwapMoveSelector<S, V, FromSolutionEntitySelector, FromSolutionEntitySelector>
{
    /// Creates a simple selector for swapping within a single entity type.
    ///
    /// # Arguments
    /// * `getter` - Typed getter function pointer
    /// * `setter` - Typed setter function pointer
    /// * `descriptor_index` - Index in the entity descriptor
    /// * `variable_name` - Name of the variable to swap
    pub fn simple(
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        descriptor_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            left_entity_selector: FromSolutionEntitySelector::new(descriptor_index),
            right_entity_selector: FromSolutionEntitySelector::new(descriptor_index),
            getter,
            setter,
            descriptor_index,
            variable_name,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, LES, RES> MoveSelector<S, SwapMove<S, V>> for SwapMoveSelector<S, V, LES, RES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    LES: EntitySelector<S>,
    RES: EntitySelector<S>,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = SwapMove<S, V>> + 'a {
        let descriptor_index = self.descriptor_index;
        let variable_name = self.variable_name;
        let getter = self.getter;
        let setter = self.setter;

        // Collect entities once — needed for triangular pairing.
        let left_entities: Vec<EntityReference> =
            self.left_entity_selector.iter(score_director).collect();
        let right_entities: Vec<EntityReference> =
            self.right_entity_selector.iter(score_director).collect();

        // Eager triangular pairing — no Rc, no shared pointers.
        let mut moves =
            Vec::with_capacity(left_entities.len() * left_entities.len().saturating_sub(1) / 2);
        for (i, left) in left_entities.iter().enumerate() {
            for right in &right_entities[i + 1..] {
                if left.descriptor_index == right.descriptor_index
                    && left.descriptor_index == descriptor_index
                {
                    moves.push(SwapMove::new(
                        left.entity_index,
                        right.entity_index,
                        getter,
                        setter,
                        variable_name,
                        descriptor_index,
                    ));
                }
            }
        }

        moves.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let left_count = self.left_entity_selector.size(score_director);
        let right_count = self.right_entity_selector.size(score_director);

        if left_count == right_count {
            left_count * left_count.saturating_sub(1) / 2
        } else {
            left_count * right_count / 2
        }
    }
}
