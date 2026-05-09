use std::cell::Cell;

pub struct GroupedScalarSelector<S> {
    group: crate::builder::context::ScalarGroupBinding<S>,
    value_candidate_limit: Option<usize>,
    max_moves_per_step: usize,
    require_hard_improvement: bool,
    next_entity_offset: Cell<usize>,
}

impl<S> GroupedScalarSelector<S> {
    pub fn new(
        group: crate::builder::context::ScalarGroupBinding<S>,
        value_candidate_limit: Option<usize>,
        max_moves_per_step: Option<usize>,
        require_hard_improvement: bool,
    ) -> Self {
        let effective_value_candidate_limit =
            value_candidate_limit.or(group.limits.value_candidate_limit);
        let effective_max_moves_per_step = max_moves_per_step
            .or(group.limits.max_moves_per_step)
            .unwrap_or(256);
        Self {
            group,
            value_candidate_limit: effective_value_candidate_limit,
            max_moves_per_step: effective_max_moves_per_step,
            require_hard_improvement,
            next_entity_offset: Cell::new(0),
        }
    }

    fn next_entity_offset(&self) -> usize {
        let offset = self.next_entity_offset.get();
        self.next_entity_offset.set(offset.wrapping_add(1));
        offset
    }

    fn limits(&self) -> crate::builder::context::ScalarGroupLimits {
        crate::builder::context::ScalarGroupLimits {
            value_candidate_limit: self.value_candidate_limit,
            group_candidate_limit: Some(self.max_moves_per_step),
            max_moves_per_step: Some(self.max_moves_per_step),
            max_augmenting_depth: None,
            max_rematch_size: None,
        }
    }
}

impl<S> std::fmt::Debug for GroupedScalarSelector<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedScalarSelector")
            .field("group_name", &self.group.group_name)
            .field("value_candidate_limit", &self.value_candidate_limit)
            .field("max_moves_per_step", &self.max_moves_per_step)
            .field("require_hard_improvement", &self.require_hard_improvement)
            .finish()
    }
}

pub struct GroupedScalarCursor<S>
where
    S: PlanningSolution + 'static,
{
    store: CandidateStore<S, ScalarMoveUnion<S, usize>>,
    next_index: usize,
}

impl<S> GroupedScalarCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn new(store: CandidateStore<S, ScalarMoveUnion<S, usize>>) -> Self {
        Self {
            store,
            next_index: 0,
        }
    }
}

impl<S> MoveCursor<S, ScalarMoveUnion<S, usize>> for GroupedScalarCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        if self.next_index >= self.store.len() {
            return None;
        }
        let id = CandidateId::new(self.next_index);
        self.next_index += 1;
        Some(id)
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, ScalarMoveUnion<S, usize>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ScalarMoveUnion<S, usize> {
        self.store.take_candidate(id)
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for GroupedScalarSelector<S>
where
    S: PlanningSolution + 'static,
{
    type Cursor<'a>
        = GroupedScalarCursor<S>
    where
        Self: 'a;

    fn open_cursor<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
    ) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        if self.max_moves_per_step == 0 {
            return GroupedScalarCursor::new(CandidateStore::new());
        }

        let candidate_provider = match self.group.kind {
            crate::builder::ScalarGroupBindingKind::Candidates { candidate_provider } => {
                candidate_provider
            }
            crate::builder::ScalarGroupBindingKind::Assignment(assignment) => {
                return self.open_assignment_cursor(score_director, assignment, context);
            }
        };
        let solution = score_director.working_solution();
        let mut store = CandidateStore::with_capacity(self.max_moves_per_step);
        let mut seen_candidates = Vec::new();
        let mut targets = std::collections::HashSet::new();

        let mut candidates = candidate_provider(solution, self.limits());
        let offset = context.start_offset(
            candidates.len(),
            0xC0A1_E5CE_AAA0_0001 ^ self.group.group_name.len() as u64,
        );
        candidates.rotate_left(offset);
        for candidate in candidates {
            if store.len() >= self.max_moves_per_step {
                break;
            }
            if candidate.edits().is_empty()
                || seen_candidates
                    .iter()
                    .any(|seen_candidate| seen_candidate == &candidate)
            {
                continue;
            }
            targets.clear();
            if candidate.edits().iter().any(|edit| {
                !targets.insert((
                    edit.descriptor_index(),
                    edit.entity_index(),
                    edit.variable_name(),
                ))
            }) {
                continue;
            }

            let Some(mov) =
                crate::phase::construction::grouped_scalar::compound_move_for_group_candidate(
                    &self.group,
                    solution,
                    &candidate,
                )
            else {
                continue;
            };
            let mov = mov.with_require_hard_improvement(self.require_hard_improvement);
            if mov.is_doable(score_director) {
                store.push(ScalarMoveUnion::CompoundScalar(mov));
                seen_candidates.push(candidate);
            }
        }

        GroupedScalarCursor::new(store)
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, _score_director: &D) -> usize {
        self.max_moves_per_step
    }
}

impl<S> GroupedScalarSelector<S>
where
    S: PlanningSolution + 'static,
{
    fn open_assignment_cursor<D: solverforge_scoring::Director<S>>(
        &self,
        score_director: &D,
        assignment: crate::builder::ScalarAssignmentBinding<S>,
        context: MoveStreamContext,
    ) -> GroupedScalarCursor<S> {
        if self.max_moves_per_step == 0 {
            return GroupedScalarCursor::new(CandidateStore::new());
        }

        let solution = score_director.working_solution();
        let entity_offset = self.next_entity_offset().wrapping_add(context.offset_seed(
            0xC0A1_E5CE_AAA0_0002 ^ self.group.group_name.len() as u64,
        ));
        let options = crate::phase::construction::grouped_scalar::ScalarAssignmentMoveOptions::for_selector(
            self.group.limits,
            self.value_candidate_limit,
            self.max_moves_per_step,
            entity_offset,
        );
        let mut store = CandidateStore::with_capacity(self.max_moves_per_step);
        for mov in crate::phase::construction::grouped_scalar::selector_assignment_moves(
            &assignment,
            solution,
            options,
        ) {
            if store.len() >= self.max_moves_per_step {
                break;
            }
            self.push_assignment_move(score_director, &mut store, mov);
        }
        GroupedScalarCursor::new(store)
    }

    fn push_assignment_move<D>(
        &self,
        score_director: &D,
        store: &mut CandidateStore<S, ScalarMoveUnion<S, usize>>,
        mov: crate::heuristic::r#move::CompoundScalarMove<S>,
    ) where
        D: solverforge_scoring::Director<S>,
    {
        let mov = mov.with_require_hard_improvement(self.require_hard_improvement);
        if mov.is_doable(score_director) {
            store.push(ScalarMoveUnion::CompoundScalar(mov));
        }
    }
}
