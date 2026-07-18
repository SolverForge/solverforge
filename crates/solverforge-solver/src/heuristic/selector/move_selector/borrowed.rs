pub use scalar_union::{ScalarChangeMoveSelector, ScalarSwapMoveSelector};

impl<S, M> Move<S> for &M
where
    S: PlanningSolution,
    M: Move<S> + ?Sized,
{
    type Undo = M::Undo;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        (**self).is_doable(score_director)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        (**self).do_move(score_director)
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        (**self).undo_move(score_director, undo);
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

    fn telemetry_label(&self) -> &'static str {
        (**self).telemetry_label()
    }

    fn requires_hard_improvement(&self) -> bool {
        (**self).requires_hard_improvement()
    }

    fn requires_score_improvement(&self) -> bool {
        (**self).requires_score_improvement()
    }

    fn tabu_signature<D: Director<S>>(
        &self,
        score_director: &D,
    ) -> crate::heuristic::r#move::MoveTabuSignature {
        (**self).tabu_signature(score_director)
    }

    fn candidate_trace_identity(&self) -> Option<crate::stats::CandidateTraceIdentity> {
        (**self).candidate_trace_identity()
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

pub enum MoveCandidateUndo<U> {
    Borrowed(U),
    Sequential { first: U, second: U },
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
    type Undo = MoveCandidateUndo<M::Undo>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::Borrowed(mov) => mov.is_doable(score_director),
            Self::Sequential(mov) => mov.is_doable(score_director),
        }
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        match self {
            Self::Borrowed(mov) => MoveCandidateUndo::Borrowed(mov.do_move(score_director)),
            Self::Sequential(mov) => {
                let first = mov.first().do_move(score_director);
                let second = mov.second().do_move(score_director);
                MoveCandidateUndo::Sequential { first, second }
            }
        }
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        match (self, undo) {
            (Self::Borrowed(mov), MoveCandidateUndo::Borrowed(undo)) => {
                mov.undo_move(score_director, undo);
            }
            (Self::Sequential(mov), MoveCandidateUndo::Sequential { first, second }) => {
                mov.second().undo_move(score_director, second);
                mov.first().undo_move(score_director, first);
            }
            _ => panic!("move candidate undo shape must match candidate shape"),
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

    fn candidate_trace_identity(&self) -> Option<crate::stats::CandidateTraceIdentity> {
        match self {
            Self::Borrowed(mov) => mov.candidate_trace_identity(),
            Self::Sequential(mov) => mov.candidate_trace_identity(),
        }
    }

    fn telemetry_label(&self) -> &'static str {
        match self {
            Self::Borrowed(mov) => mov.telemetry_label(),
            Self::Sequential(mov) => mov.telemetry_label(),
        }
    }

    fn requires_hard_improvement(&self) -> bool {
        match self {
            Self::Borrowed(mov) => mov.requires_hard_improvement(),
            Self::Sequential(mov) => mov.requires_hard_improvement(),
        }
    }

    fn requires_score_improvement(&self) -> bool {
        match self {
            Self::Borrowed(mov) => mov.requires_score_improvement(),
            Self::Sequential(mov) => mov.requires_score_improvement(),
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

/// Incremental, cursor-owned candidate storage for a single neighborhood.
///
/// `next_candidate()` constructs or discovers at most the next candidate and
/// returns a stable ID. A consumer may stop at any time: dropping the cursor
/// must release all retained candidates and any unconsumed source state without
/// requiring the remaining neighborhood to be enumerated. Implementations must
/// not rely on full exhaustion for correctness, callback delivery, or cleanup.
///
/// Candidates remain borrowable until they are released or transferred. The
/// selected candidate is consumed exactly once through `take_candidate()` or
/// `apply_owned_candidate()`; unselected live candidates remain cursor-owned.
pub trait MoveCursor<S: PlanningSolution, M: Move<S>> {
    fn next_candidate(&mut self) -> Option<CandidateId>;

    /// Pulls the next candidate while allowing expensive cursor implementations
    /// to observe solve control between units of generation work. A `None`
    /// result may mean either exhaustion or requested interruption, so callers
    /// must recheck the same control signal before resolving an exhausted scan.
    fn next_candidate_with_control<ShouldStop>(
        &mut self,
        should_stop: &mut ShouldStop,
    ) -> Option<CandidateId>
    where
        ShouldStop: FnMut() -> bool,
    {
        if should_stop() {
            None
        } else {
            self.next_candidate()
        }
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>>;

    fn take_candidate(&mut self, id: CandidateId) -> M;

    fn next_owned_candidate(&mut self) -> Option<M> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }

    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'a> fn(MoveCandidateRef<'a, S, M>) -> bool,
    ) -> Option<M> {
        loop {
            let id = self.next_candidate()?;
            let candidate = self
                .candidate(id)
                .expect("newly generated cursor candidate must remain live");
            let accepted = predicate(candidate);
            if accepted {
                return Some(self.take_candidate(id));
            }
            assert!(self.release_candidate(id));
        }
    }

    fn next_owned_candidate_inspected<T, F>(&mut self, mut inspect: F) -> Option<(M, T)>
    where
        F: for<'a> FnMut(MoveCandidateRef<'a, S, M>) -> Option<T>,
    {
        loop {
            let id = self.next_candidate()?;
            let candidate = self
                .candidate(id)
                .expect("newly generated cursor candidate must remain live");
            if let Some(metadata) = inspect(candidate) {
                return Some((self.take_candidate(id), metadata));
            }
            assert!(self.release_candidate(id));
        }
    }

    fn apply_owned_candidate<D: Director<S>>(&mut self, id: CandidateId, score_director: &mut D) {
        let mov = self.take_candidate(id);
        mov.do_move(score_director);
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        if self.candidate(id).is_none() {
            return false;
        }
        drop(self.take_candidate(id));
        true
    }

    fn selector_index(&self, _id: CandidateId) -> Option<usize> {
        None
    }
}

/// A cursor whose next pull needs one explicitly owned execution resource.
///
/// Ordinary selectors use [`MoveCursor`] directly. Runtime provider cursors
/// need a solve-owned reason arena only at the instant a reachable child is
/// pulled, however; retaining that arena in a leaf or hiding it behind
/// interior mutability would break lazy callback delivery and resource
/// ownership. Recursive composition therefore uses this private companion
/// contract and lends its resource only to the child selected by the
/// canonical union scheduler.
#[doc(hidden)]
pub trait ResourceMoveCursor<S: PlanningSolution, M: Move<S>, Resources> {
    fn next_candidate_with_resources(&mut self, resources: &mut Resources) -> Option<CandidateId>;

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>>;

    fn take_candidate(&mut self, id: CandidateId) -> M;

    fn next_owned_candidate_inspected_with_resources<T, F>(
        &mut self,
        resources: &mut Resources,
        mut inspect: F,
    ) -> Option<(M, T)>
    where
        F: for<'a> FnMut(MoveCandidateRef<'a, S, M>) -> Option<T>,
    {
        loop {
            let id = self.next_candidate_with_resources(resources)?;
            let candidate = self
                .candidate(id)
                .expect("newly generated resource cursor candidate must remain live");
            if let Some(metadata) = inspect(candidate) {
                return Some((self.take_candidate(id), metadata));
            }
            assert!(self.release_candidate(id));
        }
    }

    fn apply_owned_candidate<D: Director<S>>(&mut self, id: CandidateId, score_director: &mut D);

    fn release_candidate(&mut self, id: CandidateId) -> bool;

    fn selector_index(&self, id: CandidateId) -> Option<usize>;
}

/// Adapts an ordinary cursor to the resource-aware composition boundary.
///
/// It is deliberately explicit instead of a blanket implementation of
/// [`ResourceMoveCursor`]: compiled provider cursors need their own resource
/// lending implementation, and a blanket would overlap that specialization.
#[doc(hidden)]
pub struct UnitResourceCursor<C> {
    inner: C,
}

impl<C> UnitResourceCursor<C> {
    pub(crate) fn new(inner: C) -> Self {
        Self { inner }
    }
}

impl<S, M, C> ResourceMoveCursor<S, M, ()> for UnitResourceCursor<C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn next_candidate_with_resources(&mut self, _resources: &mut ()) -> Option<CandidateId> {
        self.inner.next_candidate()
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.inner.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> M {
        self.inner.take_candidate(id)
    }

    fn apply_owned_candidate<D: Director<S>>(&mut self, id: CandidateId, score_director: &mut D) {
        self.inner.apply_owned_candidate(id, score_director);
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }

    fn selector_index(&self, id: CandidateId) -> Option<usize> {
        self.inner.selector_index(id)
    }
}

pub struct CandidateStore<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    positions: Vec<Option<usize>>,
    live: Vec<StoredCandidate<M>>,
    _phantom: PhantomData<fn() -> S>,
}

struct StoredCandidate<M> {
    id: CandidateId,
    mov: M,
}

impl<S, M> CandidateStore<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            live: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            positions: Vec::with_capacity(capacity),
            live: Vec::with_capacity(capacity),
            _phantom: PhantomData,
        }
    }

    pub fn push(&mut self, mov: M) -> CandidateId {
        let id = CandidateId::new(self.positions.len());
        self.positions.push(Some(self.live.len()));
        self.live.push(StoredCandidate { id, mov });
        id
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = M>,
    {
        for mov in iter {
            self.push(mov);
        }
    }

    pub fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        let position = self.positions.get(id.index()).copied().flatten()?;
        self.live
            .get(position)
            .map(|stored| MoveCandidateRef::Borrowed(&stored.mov))
    }

    pub fn take_candidate(&mut self, id: CandidateId) -> M {
        let position = self
            .positions
            .get_mut(id.index())
            .and_then(Option::take)
            .expect("move cursor candidate id must remain valid");
        let stored = self.live.swap_remove(position);
        debug_assert_eq!(stored.id, id);
        if let Some(moved) = self.live.get(position) {
            self.positions[moved.id.index()] = Some(position);
        }
        stored.mov
    }

    pub fn release_candidate(&mut self, id: CandidateId) -> bool {
        if self
            .positions
            .get(id.index())
            .is_none_or(Option::is_none)
        {
            return false;
        }
        drop(self.take_candidate(id));
        true
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
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

    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
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

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
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
