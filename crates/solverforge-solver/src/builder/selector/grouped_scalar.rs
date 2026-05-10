use std::cell::Cell;

pub struct GroupedScalarSelector<S> {
    group: crate::builder::context::ScalarGroupBinding<S>,
    value_candidate_limit: Option<usize>,
    max_moves_per_step: Option<usize>,
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
        let effective_max_moves_per_step = max_moves_per_step.or(group.limits.max_moves_per_step);
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

    fn effective_max_moves_per_step(&self, solution: &S) -> usize {
        if let Some(max_moves_per_step) = self.max_moves_per_step {
            return max_moves_per_step;
        }
        match self.group.kind {
            crate::builder::ScalarGroupBindingKind::Assignment(assignment) => {
                let max_rematch_size = self.group.limits.max_rematch_size.unwrap_or(4).max(2);
                assignment
                    .entity_count(solution)
                    .saturating_mul(max_rematch_size)
                    .clamp(256, 4096)
            }
            crate::builder::ScalarGroupBindingKind::Candidates { .. } => 256,
        }
    }

    fn limits(&self, max_moves_per_step: usize) -> crate::builder::context::ScalarGroupLimits {
        crate::builder::context::ScalarGroupLimits {
            value_candidate_limit: self.value_candidate_limit,
            group_candidate_limit: Some(max_moves_per_step),
            max_moves_per_step: Some(max_moves_per_step),
            max_augmenting_depth: self.group.limits.max_augmenting_depth,
            max_rematch_size: self.group.limits.max_rematch_size,
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
    assignment_cursor:
        Option<crate::phase::construction::grouped_scalar::ScalarAssignmentMoveCursor<S>>,
    require_hard_improvement: bool,
}

impl<S> GroupedScalarCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn new(store: CandidateStore<S, ScalarMoveUnion<S, usize>>) -> Self {
        Self {
            store,
            next_index: 0,
            assignment_cursor: None,
            require_hard_improvement: false,
        }
    }

    fn assignment(
        assignment_cursor: crate::phase::construction::grouped_scalar::ScalarAssignmentMoveCursor<S>,
        require_hard_improvement: bool,
        capacity: usize,
    ) -> Self {
        Self {
            store: CandidateStore::with_capacity(capacity),
            next_index: 0,
            assignment_cursor: Some(assignment_cursor),
            require_hard_improvement,
        }
    }
}

impl<S> MoveCursor<S, ScalarMoveUnion<S, usize>> for GroupedScalarCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        if self.next_index < self.store.len() {
            let id = CandidateId::new(self.next_index);
            self.next_index += 1;
            return Some(id);
        }
        let assignment_cursor = self.assignment_cursor.as_mut()?;
        let mov = assignment_cursor
            .next_move()?
            .with_require_hard_improvement(self.require_hard_improvement);
        let id = self.store.push(ScalarMoveUnion::CompoundScalar(mov));
        self.next_index = id.index() + 1;
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
        let solution = score_director.working_solution();
        let max_moves_per_step = self.effective_max_moves_per_step(solution);
        if max_moves_per_step == 0 {
            return GroupedScalarCursor::new(CandidateStore::new());
        }

        let candidate_provider = match self.group.kind {
            crate::builder::ScalarGroupBindingKind::Candidates { candidate_provider } => {
                candidate_provider
            }
            crate::builder::ScalarGroupBindingKind::Assignment(assignment) => {
                return self.open_assignment_cursor(
                    score_director,
                    assignment,
                    context,
                    max_moves_per_step,
                );
            }
        };
        let mut store = CandidateStore::with_capacity(max_moves_per_step);
        let mut seen_candidates = Vec::new();
        let mut targets = std::collections::HashSet::new();

        let mut candidates = candidate_provider(solution, self.limits(max_moves_per_step));
        let offset = context.start_offset(
            candidates.len(),
            0xC0A1_E5CE_AAA0_0001 ^ self.group.group_name.len() as u64,
        );
        candidates.rotate_left(offset);
        for candidate in candidates {
            if store.len() >= max_moves_per_step {
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

    fn size<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> usize {
        self.effective_max_moves_per_step(score_director.working_solution())
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
        max_moves_per_step: usize,
    ) -> GroupedScalarCursor<S> {
        if max_moves_per_step == 0 {
            return GroupedScalarCursor::new(CandidateStore::new());
        }

        let solution = score_director.working_solution();
        let entity_offset = self.next_entity_offset().wrapping_add(context.offset_seed(
            0xC0A1_E5CE_AAA0_0002 ^ self.group.group_name.len() as u64,
        ));
        let options = crate::phase::construction::grouped_scalar::ScalarAssignmentMoveOptions::for_selector(
            self.group.limits,
            self.value_candidate_limit,
            max_moves_per_step,
            entity_offset,
        );
        let assignment_cursor =
            crate::phase::construction::grouped_scalar::ScalarAssignmentMoveCursor::new(
                assignment,
                solution.clone(),
                options,
            );
        GroupedScalarCursor::assignment(
            assignment_cursor,
            self.require_hard_improvement,
            max_moves_per_step,
        )
    }
}
