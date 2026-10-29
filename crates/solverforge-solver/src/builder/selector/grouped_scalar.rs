use crate::phase::construction::grouped_scalar::ScalarAssignmentState;

pub(crate) struct GroupedScalarSelector<S> {
    group: crate::builder::context::ScalarGroupBinding<S>,
    value_candidate_limit: Option<usize>,
    max_moves_per_step: Option<usize>,
    require_hard_improvement: bool,
}

impl<S> GroupedScalarSelector<S> {
    pub(crate) fn new(
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
        }
    }

    fn effective_max_moves_per_step(&self, solution: &S) -> usize {
        if let Some(max_moves_per_step) = self.max_moves_per_step {
            return max_moves_per_step;
        }
        match &self.group.kind {
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

pub(crate) struct GroupedScalarCursor<'a, S>
where
    S: PlanningSolution + 'static,
{
    store: CandidateStore<S, CompoundScalarMove<S>>,
    next_index: usize,
    group: &'a crate::builder::context::ScalarGroupBinding<S>,
    activation: Option<GroupedScalarActivation<'a, S>>,
    assignment_cursor:
        Option<crate::phase::construction::grouped_scalar::ScalarAssignmentMoveCursor<S>>,
    pooled_assignment: Option<crate::phase::construction::grouped_scalar::ScalarAssignmentState>,
    require_hard_improvement: bool,
}

struct GroupedScalarActivation<'a, S> {
    group: &'a crate::builder::context::ScalarGroupBinding<S>,
    limits: crate::builder::context::ScalarGroupLimits,
    max_moves_per_step: usize,
    context: MoveStreamContext,
    require_hard_improvement: bool,
}

impl<'a, S> GroupedScalarCursor<'a, S>
where
    S: PlanningSolution + 'static,
{
    fn new(
        group: &'a crate::builder::context::ScalarGroupBinding<S>,
        store: CandidateStore<S, CompoundScalarMove<S>>,
    ) -> Self {
        Self {
            store,
            group,
            next_index: 0,
            activation: None,
            assignment_cursor: None,
            pooled_assignment: None,
            require_hard_improvement: false,
        }
    }

    fn deferred(activation: GroupedScalarActivation<'a, S>) -> Self {
        Self {
            store: CandidateStore::with_capacity(activation.max_moves_per_step),
            group: activation.group,
            next_index: 0,
            activation: Some(activation),
            assignment_cursor: None,
            pooled_assignment: None,
            require_hard_improvement: false,
        }
    }

    fn activate(&mut self, solution: &S) {
        let Some(activation) = self.activation.take() else {
            return;
        };
        let GroupedScalarActivation {
            group,
            limits,
            max_moves_per_step,
            context,
            require_hard_improvement,
        } = activation;

        match &group.kind {
            crate::builder::ScalarGroupBindingKind::Assignment(assignment) => {
                let selection_salt =
                    0xC0A1_E5CE_AAA0_0002 ^ group.group_name.len() as u64;
                let entity_offset = if context.is_canonical() {
                    0
                } else {
                    context
                        .offset_seed(selection_salt)
                        .wrapping_add(context.step_index() as usize)
                };
                let options = crate::phase::construction::grouped_scalar::ScalarAssignmentMoveOptions::for_selector(
                    group.limits,
                    limits.value_candidate_limit,
                    max_moves_per_step,
                    entity_offset,
                );
                // Solve-owned pool: the retained base is handed into the step
                // cursor (owned there during enumeration) and re-returned to
                // the slot when the step closes. Rows whose committed value
                // moved since the last pull are re-derived in place by the
                // maintenance kernel (departures before arrivals, keys read
                // from the committed base). A missing pool builds once here.
                self.assignment_cursor = Some(
                    match self.pooled_assignment.take() {
                        Some(mut pooled)
                            if pooled.row_count() == assignment.entity_count(solution) =>
                        {
                            let changed = (0..pooled.row_count())
                                .filter(|&entity| {
                                    pooled.current_value(entity)
                                        != assignment.current_value(solution, entity)
                                })
                                .collect::<Vec<_>>();
                            if !changed.is_empty() {
                                let rows = changed.into_iter().map(|entity| {
                                    (entity, assignment.current_value(solution, entity))
                                });
                                pooled.refresh_rows(assignment, solution, rows);
                            }
                            crate::phase::construction::grouped_scalar::ScalarAssignmentMoveCursor::with_pooled_state(
                                assignment, solution, options, pooled,
                            )
                        }
                        // Structural change (entity count moved): the sparse
                        // row keys may shift globally, so rebuild from the
                        // committed solution.
                        _ => crate::phase::construction::grouped_scalar::ScalarAssignmentMoveCursor::new(
                            assignment, solution, options,
                        ),
                    },
                );
                self.require_hard_improvement = require_hard_improvement;
            }
            crate::builder::ScalarGroupBindingKind::Candidates { candidate_provider } => {
                let mut candidates = candidate_provider(solution, limits);
                context.apply_selection_order(
                    &mut candidates,
                    0xC0A1_E5CE_AAA0_0001 ^ group.group_name.len() as u64,
                );
                let mut seen_candidates = Vec::new();
                let mut targets = std::collections::HashSet::new();
                for candidate in candidates {
                    if self.store.len() >= max_moves_per_step {
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

                    let Some(mov) = crate::phase::construction::grouped_scalar::compound_move_for_group_candidate(
                        group,
                        solution,
                        &candidate,
                    ) else {
                        continue;
                    };
                    let mov = mov.with_require_hard_improvement(require_hard_improvement);
                    if mov.is_doable_on(solution) {
                        self.store.push(mov);
                        seen_candidates.push(candidate);
                    }
                }
            }
        }
    }
}

impl<S: PlanningSolution + 'static> GroupedScalarCursor<'_, S> {
    /// Hands the assignment base back to the solve-owned slot when the step
    /// closes. Steps that never activated keep the returned pool as retained
    /// state; steps after activation move the maintained base back.
    pub(crate) fn into_pooled_assignment(mut self) -> Option<ScalarAssignmentState> {
        let returned = self
            .assignment_cursor
            .take()
            .map(crate::phase::construction::grouped_scalar::ScalarAssignmentMoveCursor::into_state);
        let pooled = returned.or_else(|| self.pooled_assignment.take());
        drop(self);
        pooled
    }
}

impl<S> MoveCursor<S, CompoundScalarMove<S>> for GroupedScalarCursor<'_, S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self, solution: &S) -> Option<CandidateId> {
        self.next_candidate_with_control(solution, &mut || false)
    }

    fn next_candidate_with_control<ShouldStop>(
        &mut self,
        solution: &S,
        should_stop: &mut ShouldStop,
    ) -> Option<CandidateId>
    where
        ShouldStop: FnMut() -> bool,
    {
        if should_stop() {
            return None;
        }
        self.activate(solution);
        if self.next_index < self.store.len() {
            let id = CandidateId::new(self.next_index);
            self.next_index += 1;
            return Some(id);
        }
        let crate::builder::ScalarGroupBindingKind::Assignment(assignment) = &self.group.kind else {
            return None;
        };
        let assignment_cursor = self.assignment_cursor.as_mut()?;
        let mov = assignment_cursor
            .next_move_with_control(assignment, solution, should_stop)?
            .with_require_hard_improvement(self.require_hard_improvement);
        let id = self.store.push(mov);
        self.next_index = id.index() + 1;
        Some(id)
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, CompoundScalarMove<S>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> CompoundScalarMove<S> {
        self.store.take_candidate(id)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
    }
}

impl<S> MoveSelector<S, CompoundScalarMove<S>> for GroupedScalarSelector<S>
where
    S: PlanningSolution + 'static,
{
    type Cursor<'a>
        = GroupedScalarCursor<'a, S>
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
            return GroupedScalarCursor::new(&self.group, CandidateStore::new());
        }
        GroupedScalarCursor::deferred(GroupedScalarActivation {
            group: &self.group,
            limits: self.limits(max_moves_per_step),
            max_moves_per_step,
            context,
            require_hard_improvement: self.require_hard_improvement,
        })
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> usize {
        self.effective_max_moves_per_step(score_director.working_solution())
    }
}

impl<S: PlanningSolution + 'static> GroupedScalarSelector<S> {
    /// Compiled-open entry that owns the retained solve-state slot.
    ///
    /// The pooled assignment base moves into the cursor for the step and is
    /// handed back when the step cursor is dropped.
    pub(crate) fn open_cursor_with_retained_state<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        state: &mut Option<Box<ScalarAssignmentState>>,
        score_director: &D,
        context: MoveStreamContext,
    ) -> GroupedScalarCursor<'a, S> {
        let mut cursor = self.open_cursor_with_context(score_director, context);
        cursor.pooled_assignment = state.take().map(|boxed| *boxed);
        cursor
    }
}
