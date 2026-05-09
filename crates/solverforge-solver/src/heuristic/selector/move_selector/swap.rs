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

pub struct SwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    store: CandidateStore<S, SwapMove<S, V>>,
    left_entities: Vec<super::entity::EntityReference>,
    right_entities: Vec<super::entity::EntityReference>,
    left_offset: usize,
    right_offset: usize,
    getter: fn(&S, usize, usize) -> Option<V>,
    setter: fn(&mut S, usize, usize, Option<V>),
    descriptor_index: usize,
    variable_index: usize,
    variable_name: &'static str,
}

impl<S, V> SwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn new(
        left_entities: Vec<super::entity::EntityReference>,
        right_entities: Vec<super::entity::EntityReference>,
        getter: fn(&S, usize, usize) -> Option<V>,
        setter: fn(&mut S, usize, usize, Option<V>),
        descriptor_index: usize,
        variable_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            left_entities,
            right_entities,
            left_offset: 0,
            right_offset: 0,
            getter,
            setter,
            descriptor_index,
            variable_index,
            variable_name,
        }
    }
}

impl<S, V> MoveCursor<S, SwapMove<S, V>> for SwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        while self.left_offset < self.left_entities.len() {
            while self.right_offset < self.right_entities.len() {
                let left_entity_ref = self.left_entities[self.left_offset];
                let right_entity_ref = self.right_entities[self.right_offset];
                self.right_offset += 1;

                if left_entity_ref.entity_index >= right_entity_ref.entity_index {
                    continue;
                }

                return Some(self.store.push(SwapMove::new(
                    left_entity_ref.entity_index,
                    right_entity_ref.entity_index,
                    self.getter,
                    self.setter,
                    self.variable_index,
                    self.variable_name,
                    self.descriptor_index,
                )));
            }

            self.left_offset += 1;
            self.right_offset = 0;
        }

        None
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, SwapMove<S, V>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> SwapMove<S, V> {
        self.store.take_candidate(id)
    }
}

impl<S, V> Iterator for SwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Item = SwapMove<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
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
        = SwapMoveCursor<S, V>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        let mut right_entities: Vec<_> = self.right_entity_selector.iter(score_director).collect();
        let mut left_entities: Vec<_> = self.left_entity_selector.iter(score_director).collect();
        let salt = ((self.descriptor_index as u64) << 32) ^ self.variable_index as u64;
        let left_start = context.start_offset(left_entities.len(), 0x5A09_0000_0000_0001 ^ salt);
        let right_start = context.start_offset(right_entities.len(), 0x5A09_0000_0000_0002 ^ salt);
        left_entities.rotate_left(left_start);
        right_entities.rotate_left(right_start);
        SwapMoveCursor::new(
            left_entities,
            right_entities,
            self.getter,
            self.setter,
            self.descriptor_index,
            self.variable_index,
            self.variable_name,
        )
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let left_count = self.left_entity_selector.iter(score_director).count();
        let right_count = self.right_entity_selector.iter(score_director).count();
        left_count.saturating_mul(right_count.saturating_sub(1)) / 2
    }
}
