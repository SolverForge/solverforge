//! Typed move selectors for zero-allocation move generation.
//!
//! Typed move selectors yield concrete move types directly, enabling
//! monomorphization and arena allocation.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::{ChangeMove, EitherMove, Move, SwapMove};

use super::entity::{EntityReference, EntitySelector, FromSolutionEntitySelector};
use super::typed_value::{StaticTypedValueSelector, TypedValueSelector};

/// A typed move selector that yields moves of type `M` directly.
///
/// Unlike erased selectors, this returns concrete moves inline,
/// eliminating heap allocation per move.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
pub trait MoveSelector<S: PlanningSolution, M: Move<S>>: Send + Debug {
    /// Returns an iterator over typed moves.
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = M> + 'a;

    /// Returns the approximate number of moves.
    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize;

    /// Returns true if this selector may return the same move multiple times.
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
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V: Debug, ES: Debug, VS: Debug> Debug for ChangeMoveSelector<S, V, ES, VS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("value_selector", &self.value_selector)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
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
            _phantom: PhantomData,
        }
    }
}

impl<S: PlanningSolution, V: Clone + Send + Sync + Debug + 'static>
    ChangeMoveSelector<S, V, FromSolutionEntitySelector, StaticTypedValueSelector<S, V>>
{
    /// Creates a simple selector with static values.
    pub fn simple(
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        descriptor_index: usize,
        variable_name: &'static str,
        values: Vec<V>,
    ) -> Self {
        Self {
            entity_selector: FromSolutionEntitySelector::new(descriptor_index),
            value_selector: StaticTypedValueSelector::new(values),
            getter,
            setter,
            descriptor_index,
            variable_name,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, ES, VS> MoveSelector<S, ChangeMove<S, V>> for ChangeMoveSelector<S, V, ES, VS>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
    VS: TypedValueSelector<S, V>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = ChangeMove<S, V>> + 'a {
        let descriptor_index = self.descriptor_index;
        let variable_name = self.variable_name;
        let getter = self.getter;
        let setter = self.setter;
        let value_selector = &self.value_selector;

        // Lazy iteration: O(1) per .next() call, no upfront allocation
        self.entity_selector
            .iter(score_director)
            .flat_map(move |entity_ref| {
                value_selector
                    .iter_typed(
                        score_director,
                        entity_ref.descriptor_index,
                        entity_ref.entity_index,
                    )
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
            })
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        let entity_count = self.entity_selector.size(score_director);
        if entity_count == 0 {
            return 0;
        }

        if let Some(entity_ref) = self.entity_selector.iter(score_director).next() {
            let value_count = self.value_selector.size(
                score_director,
                entity_ref.descriptor_index,
                entity_ref.entity_index,
            );
            entity_count * value_count
        } else {
            0
        }
    }
}

/// A swap move selector that generates `SwapMove` instances.
///
/// Uses typed function pointers for zero-erasure access to variable values.
pub struct SwapMoveSelector<S, V, LES, RES> {
    left_entity_selector: LES,
    right_entity_selector: RES,
    /// Typed getter function pointer - zero erasure.
    getter: fn(&S, usize) -> Option<V>,
    /// Typed setter function pointer - zero erasure.
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
    /// Creates a new swap move selector with typed function pointers.
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
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
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

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        let left_count = self.left_entity_selector.size(score_director);
        let right_count = self.right_entity_selector.size(score_director);

        if left_count == right_count {
            left_count * left_count.saturating_sub(1) / 2
        } else {
            left_count * right_count / 2
        }
    }
}

// ---------------------------------------------------------------------------
// EitherMove adaptor selectors
// ---------------------------------------------------------------------------

/// Wraps a `ChangeMoveSelector` to yield `EitherMove::Change`.
pub struct EitherChangeMoveSelector<S, V, ES, VS> {
    inner: ChangeMoveSelector<S, V, ES, VS>,
}

impl<S, V: Debug, ES: Debug, VS: Debug> Debug for EitherChangeMoveSelector<S, V, ES, VS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EitherChangeMoveSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S: PlanningSolution, V: Clone + Send + Sync + Debug + 'static>
    EitherChangeMoveSelector<S, V, FromSolutionEntitySelector, StaticTypedValueSelector<S, V>>
{
    /// Creates a simple selector that yields `EitherMove::Change` variants.
    pub fn simple(
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        descriptor_index: usize,
        variable_name: &'static str,
        values: Vec<V>,
    ) -> Self {
        Self {
            inner: ChangeMoveSelector::simple(
                getter,
                setter,
                descriptor_index,
                variable_name,
                values,
            ),
        }
    }
}

impl<S, V, ES, VS> MoveSelector<S, EitherMove<S, V>> for EitherChangeMoveSelector<S, V, ES, VS>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
    VS: TypedValueSelector<S, V>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = EitherMove<S, V>> + 'a {
        self.inner
            .iter_moves(score_director)
            .map(EitherMove::Change)
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }
}

/// Wraps a `SwapMoveSelector` to yield `EitherMove::Swap`.
pub struct EitherSwapMoveSelector<S, V, LES, RES> {
    inner: SwapMoveSelector<S, V, LES, RES>,
}

impl<S, V: Debug, LES: Debug, RES: Debug> Debug for EitherSwapMoveSelector<S, V, LES, RES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EitherSwapMoveSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S: PlanningSolution, V>
    EitherSwapMoveSelector<S, V, FromSolutionEntitySelector, FromSolutionEntitySelector>
{
    /// Creates a simple selector that yields `EitherMove::Swap` variants.
    pub fn simple(
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        descriptor_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            inner: SwapMoveSelector::simple(getter, setter, descriptor_index, variable_name),
        }
    }
}

impl<S, V, LES, RES> MoveSelector<S, EitherMove<S, V>> for EitherSwapMoveSelector<S, V, LES, RES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    LES: EntitySelector<S>,
    RES: EntitySelector<S>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = EitherMove<S, V>> + 'a {
        self.inner.iter_moves(score_director).map(EitherMove::Swap)
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }
}

#[cfg(test)]
#[path = "typed_move_selector_tests.rs"]
mod tests;
