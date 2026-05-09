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

struct ChangeEntityValues<V> {
    entity_ref: super::entity::EntityReference,
    values: Vec<V>,
    current_assigned: bool,
}

pub struct ChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    store: CandidateStore<S, ChangeMove<S, V>>,
    entity_values: Vec<ChangeEntityValues<V>>,
    entity_offset: usize,
    value_offset: usize,
    getter: fn(&S, usize, usize) -> Option<V>,
    setter: fn(&mut S, usize, usize, Option<V>),
    descriptor_index: usize,
    variable_index: usize,
    variable_name: &'static str,
    allows_unassigned: bool,
}

impl<S, V> ChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn new(
        entity_values: Vec<ChangeEntityValues<V>>,
        getter: fn(&S, usize, usize) -> Option<V>,
        setter: fn(&mut S, usize, usize, Option<V>),
        descriptor_index: usize,
        variable_index: usize,
        variable_name: &'static str,
        allows_unassigned: bool,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            entity_values,
            entity_offset: 0,
            value_offset: 0,
            getter,
            setter,
            descriptor_index,
            variable_index,
            variable_name,
            allows_unassigned,
        }
    }
}

impl<S, V> MoveCursor<S, ChangeMove<S, V>> for ChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        while self.entity_offset < self.entity_values.len() {
            let entity_values = &self.entity_values[self.entity_offset];
            if self.value_offset < entity_values.values.len() {
                let value = entity_values.values[self.value_offset].clone();
                self.value_offset += 1;
                return Some(self.store.push(ChangeMove::new(
                    entity_values.entity_ref.entity_index,
                    Some(value),
                    self.getter,
                    self.setter,
                    self.variable_index,
                    self.variable_name,
                    self.descriptor_index,
                )));
            }

            let to_none_offset = entity_values.values.len();
            if self.allows_unassigned
                && entity_values.current_assigned
                && self.value_offset == to_none_offset
            {
                self.value_offset += 1;
                return Some(self.store.push(ChangeMove::new(
                    entity_values.entity_ref.entity_index,
                    None,
                    self.getter,
                    self.setter,
                    self.variable_index,
                    self.variable_name,
                    self.descriptor_index,
                )));
            }

            self.entity_offset += 1;
            self.value_offset = 0;
        }

        None
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, ChangeMove<S, V>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ChangeMove<S, V> {
        self.store.take_candidate(id)
    }
}

impl<S, V> Iterator for ChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Item = ChangeMove<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
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
        = ChangeMoveCursor<S, V>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let solution = score_director.working_solution();
        let entity_values = self
            .entity_selector
            .iter(score_director)
            .map(|entity_ref| {
                let current_assigned = (self.getter)(
                    solution,
                    entity_ref.entity_index,
                    self.variable_index,
                )
                .is_some();
                let values = self.value_selector.iter(
                    score_director,
                    entity_ref.descriptor_index,
                    entity_ref.entity_index,
                ).collect();
                ChangeEntityValues {
                    entity_ref,
                    values,
                    current_assigned,
                }
            })
            .collect();
        ChangeMoveCursor::new(
            entity_values,
            self.getter,
            self.setter,
            self.descriptor_index,
            self.variable_index,
            self.variable_name,
            self.allows_unassigned,
        )
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
