/* Typed move selectors for zero-erasure move generation.

Selectors now expose cursor-owned storage plus borrowable candidates.
The solver evaluates candidates by reference and only takes ownership of the
selected move once the forager commits to an index.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{ChangeMove, Move, MoveArena, SequentialCompositeMoveRef, SwapMove};

use super::entity::{EntitySelector, FromSolutionEntitySelector};
use super::value_selector::{StaticValueSelector, ValueSelector};

mod either;

pub use either::{ScalarChangeMoveSelector, ScalarSwapMoveSelector};

impl<S, M> Move<S> for &M
where
    S: PlanningSolution,
    M: Move<S> + ?Sized,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        (**self).is_doable(score_director)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
        (**self).do_move(score_director)
    }

    fn descriptor_index(&self) -> usize {
        (**self).descriptor_index()
    }

    fn entity_indices(&self) -> &[usize] {
        (**self).entity_indices()
    }

    fn variable_name(&self) -> &str {
        (**self).variable_name()
    }

    fn tabu_signature<D: Director<S>>(
        &self,
        score_director: &D,
    ) -> crate::heuristic::r#move::MoveTabuSignature {
        (**self).tabu_signature(score_director)
    }
}

pub enum MoveCandidateRef<'a, S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    Borrowed(&'a M),
    Sequential(SequentialCompositeMoveRef<'a, S, M>),
}

impl<S, M> Debug for MoveCandidateRef<'_, S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Borrowed(_) => write!(f, "MoveCandidateRef::Borrowed(..)"),
            Self::Sequential(mov) => write!(f, "MoveCandidateRef::Sequential({mov:?})"),
        }
    }
}

impl<S, M> Move<S> for MoveCandidateRef<'_, S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::Borrowed(mov) => mov.is_doable(score_director),
            Self::Sequential(mov) => mov.is_doable(score_director),
        }
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
        match self {
            Self::Borrowed(mov) => mov.do_move(score_director),
            Self::Sequential(mov) => mov.do_move(score_director),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Borrowed(mov) => mov.descriptor_index(),
            Self::Sequential(mov) => mov.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Borrowed(mov) => mov.entity_indices(),
            Self::Sequential(mov) => mov.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Borrowed(mov) => mov.variable_name(),
            Self::Sequential(mov) => mov.variable_name(),
        }
    }

    fn tabu_signature<D: Director<S>>(
        &self,
        score_director: &D,
    ) -> crate::heuristic::r#move::MoveTabuSignature {
        match self {
            Self::Borrowed(mov) => mov.tabu_signature(score_director),
            Self::Sequential(mov) => mov.tabu_signature(score_director),
        }
    }
}

pub trait MoveCursor<S: PlanningSolution, M: Move<S>> {
    fn next_candidate(&mut self) -> Option<(usize, MoveCandidateRef<'_, S, M>)>;

    fn candidate(&self, index: usize) -> Option<MoveCandidateRef<'_, S, M>>;

    fn take_candidate(&mut self, index: usize) -> M;
}

pub struct ArenaMoveCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    moves: Vec<Option<M>>,
    next_index: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M> ArenaMoveCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub fn new() -> Self {
        Self {
            moves: Vec::new(),
            next_index: 0,
            _phantom: PhantomData,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            moves: Vec::with_capacity(capacity),
            next_index: 0,
            _phantom: PhantomData,
        }
    }

    pub fn from_moves<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = M>,
    {
        let mut cursor = Self::new();
        cursor.extend(iter);
        cursor
    }

    pub fn push(&mut self, mov: M) {
        self.moves.push(Some(mov));
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = M>,
    {
        self.moves.extend(iter.into_iter().map(Some));
    }
}

impl<S, M> Default for ArenaMoveCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S, M> Debug for ArenaMoveCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S> + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArenaMoveCursor")
            .field("move_count", &self.moves.len())
            .field("next_index", &self.next_index)
            .finish()
    }
}

impl<S, M> MoveCursor<S, M> for ArenaMoveCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn next_candidate(&mut self) -> Option<(usize, MoveCandidateRef<'_, S, M>)> {
        while self.next_index < self.moves.len() {
            let index = self.next_index;
            self.next_index += 1;
            if let Some(mov) = self.moves[index].as_ref() {
                return Some((index, MoveCandidateRef::Borrowed(mov)));
            }
        }
        None
    }

    fn candidate(&self, index: usize) -> Option<MoveCandidateRef<'_, S, M>> {
        self.moves
            .get(index)
            .and_then(|mov| mov.as_ref())
            .map(MoveCandidateRef::Borrowed)
    }

    fn take_candidate(&mut self, index: usize) -> M {
        self.moves
            .get_mut(index)
            .and_then(Option::take)
            .expect("move cursor candidate index must remain valid")
    }
}

impl<S, M> Iterator for ArenaMoveCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    type Item = M;

    fn next(&mut self) -> Option<Self::Item> {
        let index = {
            let (index, _) = self.next_candidate()?;
            index
        };
        Some(self.take_candidate(index))
    }
}

pub(crate) fn collect_cursor_indices<S, M, C>(cursor: &mut C) -> Vec<usize>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    let mut indices = Vec::new();
    while let Some((index, _)) = cursor.next_candidate() {
        indices.push(index);
    }
    indices
}

pub struct MoveSelectorIter<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    cursor: C,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, C> MoveSelectorIter<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn new(cursor: C) -> Self {
        Self {
            cursor,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, C> Iterator for MoveSelectorIter<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    type Item = M;

    fn next(&mut self) -> Option<Self::Item> {
        let index = {
            let (index, _) = self.cursor.next_candidate()?;
            index
        };
        Some(self.cursor.take_candidate(index))
    }
}

/// A typed move selector that yields stable candidate indices plus borrowable
/// move views. Ownership is transferred only via `take_candidate`.
pub trait MoveSelector<S: PlanningSolution, M: Move<S>>: Send + Debug {
    type Cursor<'a>: MoveCursor<S, M> + 'a
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a>;

    fn iter_moves<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> MoveSelectorIter<S, M, Self::Cursor<'a>> {
        MoveSelectorIter::new(self.open_cursor(score_director))
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize;

    fn append_moves<D: Director<S>>(&self, score_director: &D, arena: &mut MoveArena<M>) {
        let mut cursor = self.open_cursor(score_director);
        for index in collect_cursor_indices::<S, M, _>(&mut cursor) {
            arena.push(cursor.take_candidate(index));
        }
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}

/// A change move selector that generates `ChangeMove` instances.
pub struct ChangeMoveSelector<S, V, ES, VS> {
    entity_selector: ES,
    value_selector: VS,
    getter: fn(&S, usize, usize) -> Option<V>,
    setter: fn(&mut S, usize, usize, Option<V>),
    descriptor_index: usize,
    variable_index: usize,
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
            .field("variable_index", &self.variable_index)
            .field("variable_name", &self.variable_name)
            .field("allows_unassigned", &self.allows_unassigned)
            .finish()
    }
}

impl<S: PlanningSolution, V: Clone, ES, VS> ChangeMoveSelector<S, V, ES, VS> {
    pub fn new(
        entity_selector: ES,
        value_selector: VS,
        getter: fn(&S, usize, usize) -> Option<V>,
        setter: fn(&mut S, usize, usize, Option<V>),
        descriptor_index: usize,
        variable_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            entity_selector,
            value_selector,
            getter,
            setter,
            descriptor_index,
            variable_index,
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
        getter: fn(&S, usize, usize) -> Option<V>,
        setter: fn(&mut S, usize, usize, Option<V>),
        descriptor_index: usize,
        variable_index: usize,
        variable_name: &'static str,
        values: Vec<V>,
    ) -> Self {
        Self {
            entity_selector: FromSolutionEntitySelector::new(descriptor_index),
            value_selector: StaticValueSelector::new(values),
            getter,
            setter,
            descriptor_index,
            variable_index,
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
    type Cursor<'a>
        = ArenaMoveCursor<S, ChangeMove<S, V>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let descriptor_index = self.descriptor_index;
        let variable_index = self.variable_index;
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
                let current_assigned =
                    getter(solution, entity_ref.entity_index, variable_index).is_some();
                let values = value_selector.iter(
                    score_director,
                    entity_ref.descriptor_index,
                    entity_ref.entity_index,
                );
                (entity_ref, values, current_assigned)
            })
            .collect();

        let iter =
            entity_values
                .into_iter()
                .flat_map(move |(entity_ref, values, current_assigned)| {
                    let to_none = (allows_unassigned && current_assigned).then(|| {
                        ChangeMove::new(
                            entity_ref.entity_index,
                            None,
                            getter,
                            setter,
                            variable_index,
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
                                variable_index,
                                variable_name,
                                descriptor_index,
                            )
                        })
                        .chain(to_none)
                });
        ArenaMoveCursor::from_moves(iter)
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
                            self.variable_index,
                        )
                        .is_some(),
                )
            })
            .sum()
    }
}

/// A swap move selector that generates `SwapMove` instances.
pub struct SwapMoveSelector<S, V, LES, RES> {
    left_entity_selector: LES,
    right_entity_selector: RES,
    getter: fn(&S, usize, usize) -> Option<V>,
    setter: fn(&mut S, usize, usize, Option<V>),
    descriptor_index: usize,
    variable_index: usize,
    variable_name: &'static str,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V, LES: Debug, RES: Debug> Debug for SwapMoveSelector<S, V, LES, RES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwapMoveSelector")
            .field("left_entity_selector", &self.left_entity_selector)
            .field("right_entity_selector", &self.right_entity_selector)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_index", &self.variable_index)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S: PlanningSolution, V, LES, RES> SwapMoveSelector<S, V, LES, RES> {
    pub fn new(
        left_entity_selector: LES,
        right_entity_selector: RES,
        getter: fn(&S, usize, usize) -> Option<V>,
        setter: fn(&mut S, usize, usize, Option<V>),
        descriptor_index: usize,
        variable_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            left_entity_selector,
            right_entity_selector,
            getter,
            setter,
            descriptor_index,
            variable_index,
            variable_name,
            _phantom: PhantomData,
        }
    }
}

impl<S: PlanningSolution, V>
    SwapMoveSelector<S, V, FromSolutionEntitySelector, FromSolutionEntitySelector>
{
    pub fn simple(
        getter: fn(&S, usize, usize) -> Option<V>,
        setter: fn(&mut S, usize, usize, Option<V>),
        descriptor_index: usize,
        variable_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            left_entity_selector: FromSolutionEntitySelector::new(descriptor_index),
            right_entity_selector: FromSolutionEntitySelector::new(descriptor_index),
            getter,
            setter,
            descriptor_index,
            variable_index,
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
    type Cursor<'a>
        = ArenaMoveCursor<S, SwapMove<S, V>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let getter = self.getter;
        let setter = self.setter;
        let variable_index = self.variable_index;
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;
        let right_entities: Vec<_> = self.right_entity_selector.iter(score_director).collect();
        let mut moves = Vec::new();
        for left_entity_ref in self.left_entity_selector.iter(score_director) {
            for right_entity_ref in &right_entities {
                if left_entity_ref.entity_index < right_entity_ref.entity_index {
                    moves.push(SwapMove::new(
                        left_entity_ref.entity_index,
                        right_entity_ref.entity_index,
                        getter,
                        setter,
                        variable_index,
                        variable_name,
                        descriptor_index,
                    ));
                }
            }
        }
        ArenaMoveCursor::from_moves(moves)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let left_count = self.left_entity_selector.iter(score_director).count();
        let right_count = self.right_entity_selector.iter(score_director).count();
        left_count.saturating_mul(right_count.saturating_sub(1)) / 2
    }
}
