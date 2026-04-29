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

    fn requires_hard_improvement(&self) -> bool {
        (**self).requires_hard_improvement()
    }

    fn tabu_signature<D: Director<S>>(
        &self,
        score_director: &D,
    ) -> crate::heuristic::r#move::MoveTabuSignature {
        (**self).tabu_signature(score_director)
    }

    fn for_each_affected_entity(
        &self,
        visitor: &mut dyn FnMut(crate::heuristic::r#move::MoveAffectedEntity<'_>),
    ) {
        (**self).for_each_affected_entity(visitor);
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CandidateId(usize);

impl CandidateId {
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    pub fn index(self) -> usize {
        self.0
    }
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

    fn requires_hard_improvement(&self) -> bool {
        match self {
            Self::Borrowed(mov) => mov.requires_hard_improvement(),
            Self::Sequential(mov) => mov.requires_hard_improvement(),
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

    fn for_each_affected_entity(
        &self,
        visitor: &mut dyn FnMut(crate::heuristic::r#move::MoveAffectedEntity<'_>),
    ) {
        match self {
            Self::Borrowed(mov) => mov.for_each_affected_entity(visitor),
            Self::Sequential(mov) => mov.for_each_affected_entity(visitor),
        }
    }
}

pub trait MoveCursor<S: PlanningSolution, M: Move<S>> {
    fn next_candidate(&mut self) -> Option<CandidateId>;

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>>;

    fn take_candidate(&mut self, id: CandidateId) -> M;

    fn selector_index(&self, _id: CandidateId) -> Option<usize> {
        None
    }
}

pub struct CandidateStore<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    moves: Vec<Option<M>>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M> CandidateStore<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub fn new() -> Self {
        Self {
            moves: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            moves: Vec::with_capacity(capacity),
            _phantom: PhantomData,
        }
    }

    pub fn push(&mut self, mov: M) -> CandidateId {
        let id = CandidateId::new(self.moves.len());
        self.moves.push(Some(mov));
        id
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = M>,
    {
        self.moves.extend(iter.into_iter().map(Some));
    }

    pub fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.moves
            .get(id.index())
            .and_then(|mov| mov.as_ref())
            .map(MoveCandidateRef::Borrowed)
    }

    pub fn take_candidate(&mut self, id: CandidateId) -> M {
        self.moves
            .get_mut(id.index())
            .and_then(Option::take)
            .expect("move cursor candidate id must remain valid")
    }

    pub fn len(&self) -> usize {
        self.moves.len()
    }

    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }
}

impl<S, M> Default for CandidateStore<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct ArenaMoveCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    store: CandidateStore<S, M>,
    next_index: usize,
}

impl<S, M> ArenaMoveCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub fn new() -> Self {
        Self {
            store: CandidateStore::new(),
            next_index: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            store: CandidateStore::with_capacity(capacity),
            next_index: 0,
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

    pub fn push(&mut self, mov: M) -> CandidateId {
        self.store.push(mov)
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = M>,
    {
        self.store.extend(iter);
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
            .field("move_count", &self.store.len())
            .field("next_index", &self.next_index)
            .finish()
    }
}

impl<S, M> MoveCursor<S, M> for ArenaMoveCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        while self.next_index < self.store.len() {
            let id = CandidateId::new(self.next_index);
            self.next_index += 1;
            if self.store.candidate(id).is_some() {
                return Some(id);
            }
        }
        None
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> M {
        self.store.take_candidate(id)
    }
}

impl<S, M> Iterator for ArenaMoveCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    type Item = M;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
}

pub(crate) fn collect_cursor_indices<S, M, C>(cursor: &mut C) -> Vec<CandidateId>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    let mut indices = Vec::new();
    while let Some(id) = cursor.next_candidate() {
        indices.push(id);
    }
    indices
}
