/// A queued entity placer that processes entities in order.
///
/// For each uninitialized entity, generates change moves for all possible values.
/// Uses concrete function pointers for zero-erasure access.
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
    // Concrete getter function pointer.
    getter: fn(&S, usize, usize) -> Option<V>,
    // Concrete setter function pointer.
    setter: fn(&mut S, usize, usize, Option<V>),
    variable_index: usize,
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
            variable_index,
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

struct QueuedLiveCandidateStore<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    primary: Option<(CandidateId, ChangeMove<S, V>)>,
    overflow: Vec<(CandidateId, ChangeMove<S, V>)>,
    next_id: usize,
}

impl<S, V> QueuedLiveCandidateStore<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn new() -> Self {
        Self {
            primary: None,
            overflow: Vec::new(),
            next_id: 0,
        }
    }

    fn push(&mut self, mov: ChangeMove<S, V>) -> CandidateId {
        let id = CandidateId::new(self.next_id);
        self.next_id += 1;
        if self.primary.is_none() {
            self.primary = Some((id, mov));
        } else {
            self.overflow.push((id, mov));
        }
        id
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ChangeMove<S, V>>> {
        self.primary
            .as_ref()
            .filter(|(candidate_id, _)| *candidate_id == id)
            .map(|(_, mov)| MoveCandidateRef::Borrowed(mov))
            .or_else(|| {
                self.overflow
                    .iter()
                    .find(|(candidate_id, _)| *candidate_id == id)
                    .map(|(_, mov)| MoveCandidateRef::Borrowed(mov))
            })
    }

    fn take_candidate(&mut self, id: CandidateId) -> ChangeMove<S, V> {
        if self
            .primary
            .as_ref()
            .is_some_and(|(candidate_id, _)| *candidate_id == id)
        {
            let (_, mov) = self
                .primary
                .take()
                .expect("checked queued construction candidate must remain live");
            self.primary = self.overflow.pop();
            return mov;
        }
        let index = self
            .overflow
            .iter()
            .position(|(candidate_id, _)| *candidate_id == id)
            .expect("queued construction candidate id must remain live");
        self.overflow.swap_remove(index).1
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        if self.candidate(id).is_none() {
            return false;
        }
        drop(self.take_candidate(id));
        true
    }
}

pub struct QueuedPlacementCandidateCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    store: QueuedLiveCandidateStore<S, V>,
    values: std::vec::IntoIter<V>,
    entity_index: usize,
    getter: fn(&S, usize, usize) -> Option<V>,
    setter: fn(&mut S, usize, usize, Option<V>),
    variable_index: usize,
    variable_name: &'static str,
    descriptor_index: usize,
}

impl<S, V> MoveCursor<S, ChangeMove<S, V>> for QueuedPlacementCandidateCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        let value = self.values.next()?;
        Some(self.store.push(ChangeMove::new(
            self.entity_index,
            Some(value),
            self.getter,
            self.setter,
            self.variable_index,
            self.variable_name,
            self.descriptor_index,
        )))
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ChangeMove<S, V>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ChangeMove<S, V> {
        self.store.take_candidate(id)
    }

    #[inline(always)]
    fn next_owned_candidate(&mut self) -> Option<ChangeMove<S, V>> {
        let value = self.values.next()?;
        Some(ChangeMove::new(
            self.entity_index,
            Some(value),
            self.getter,
            self.setter,
            self.variable_index,
            self.variable_name,
            self.descriptor_index,
        ))
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
    }
}

pub struct QueuedEntityPlacerCursor<'a, S, V, ES, VS>
where
    S: PlanningSolution,
    ES: EntitySelector<S>,
    VS: ValueSelector<S, V>,
{
    placer: &'a QueuedEntityPlacer<S, V, ES, VS>,
    entities: std::vec::IntoIter<EntityReference>,
}

impl<S, V, ES, VS> EntityPlacerCursor<S, ChangeMove<S, V>>
    for QueuedEntityPlacerCursor<'_, S, V, ES, VS>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
    VS: ValueSelector<S, V>,
{
    type CandidateCursor = QueuedPlacementCandidateCursor<S, V>;

    fn next_placement<D, IsCompleted, ShouldStop>(
        &mut self,
        score_director: &D,
        mut is_completed: IsCompleted,
        mut should_stop: ShouldStop,
    ) -> Option<Placement<S, ChangeMove<S, V>, Self::CandidateCursor>>
    where
        D: Director<S>,
        IsCompleted: FnMut(&Placement<S, ChangeMove<S, V>, Self::CandidateCursor>) -> bool,
        ShouldStop: FnMut() -> bool,
    {
        while !should_stop() {
            let entity_ref = self.entities.next()?;
            if (self.placer.getter)(
                score_director.working_solution(),
                entity_ref.entity_index,
                self.placer.variable_index,
            )
            .is_some()
            {
                continue;
            }
            let values = self
                .placer
                .value_selector
                .iter(
                    score_director,
                    entity_ref.descriptor_index,
                    entity_ref.entity_index,
                )
                .collect::<Vec<_>>();
            if values.is_empty() {
                continue;
            }
            let placement = Placement::new(
                entity_ref,
                QueuedPlacementCandidateCursor {
                    store: QueuedLiveCandidateStore::new(),
                    values: values.into_iter(),
                    entity_index: entity_ref.entity_index,
                    getter: self.placer.getter,
                    setter: self.placer.setter,
                    variable_index: self.placer.variable_index,
                    variable_name: self.placer.variable_name,
                    descriptor_index: self.placer.descriptor_index,
                },
            )
            .with_keep_current_legal(self.placer.allows_unassigned);
            if !is_completed(&placement) {
                return Some(placement);
            }
        }
        None
    }
}

impl<S, V, ES, VS> EntityPlacer<S, ChangeMove<S, V>> for QueuedEntityPlacer<S, V, ES, VS>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
    VS: ValueSelector<S, V>,
{
    type Cursor<'a>
        = QueuedEntityPlacerCursor<'a, S, V, ES, VS>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        QueuedEntityPlacerCursor {
            placer: self,
            entities: self
                .entity_selector
                .iter(score_director)
                .collect::<Vec<_>>()
                .into_iter(),
        }
    }
}
