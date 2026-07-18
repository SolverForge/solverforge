pub struct GroupedScalarSelector<S> {
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

pub struct GroupedScalarCursor<S>
where
    S: PlanningSolution + 'static,
{
    store: CandidateStore<S, ScalarMoveUnion<S, usize>>,
    next_index: usize,
    activation: Option<GroupedScalarActivation<S>>,
    assignment_cursor:
        Option<crate::phase::construction::grouped_scalar::ScalarAssignmentMoveCursor<S>>,
    require_hard_improvement: bool,
}

struct GroupedScalarActivation<S> {
    group: crate::builder::context::ScalarGroupBinding<S>,
    solution: S,
    limits: crate::builder::context::ScalarGroupLimits,
    max_moves_per_step: usize,
    context: MoveStreamContext,
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
            activation: None,
            assignment_cursor: None,
            require_hard_improvement: false,
        }
    }

    fn deferred(activation: GroupedScalarActivation<S>) -> Self {
        Self {
            store: CandidateStore::with_capacity(activation.max_moves_per_step),
            next_index: 0,
            activation: Some(activation),
            assignment_cursor: None,
            require_hard_improvement: false,
        }
    }

    fn activate(&mut self) {
        let Some(activation) = self.activation.take() else {
            return;
        };
        let GroupedScalarActivation {
            group,
            solution,
            limits,
            max_moves_per_step,
            context,
            require_hard_improvement,
        } = activation;

        match &group.kind {
            crate::builder::ScalarGroupBindingKind::Assignment(assignment) => {
                let options = crate::phase::construction::grouped_scalar::ScalarAssignmentMoveOptions::for_selector(
                    group.limits,
                    limits.value_candidate_limit,
                    max_moves_per_step,
                    context,
                );
                self.assignment_cursor = Some(
                    crate::phase::construction::grouped_scalar::ScalarAssignmentMoveCursor::new(
                        assignment.clone(),
                        solution,
                        options,
                    ),
                );
                self.require_hard_improvement = require_hard_improvement;
            }
            crate::builder::ScalarGroupBindingKind::Candidates { candidate_provider } => {
                let mut candidates = candidate_provider(&solution, limits);
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
                        &group,
                        &solution,
                        &candidate,
                    ) else {
                        continue;
                    };
                    let mov = mov.with_require_hard_improvement(require_hard_improvement);
                    if mov.is_doable_on(&solution) {
                        self.store.push(ScalarMoveUnion::CompoundScalar(mov));
                        seen_candidates.push(candidate);
                    }
                }
            }
        }
    }
}

impl<S> MoveCursor<S, ScalarMoveUnion<S, usize>> for GroupedScalarCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.next_candidate_with_control(&mut || false)
    }

    fn next_candidate_with_control<ShouldStop>(
        &mut self,
        should_stop: &mut ShouldStop,
    ) -> Option<CandidateId>
    where
        ShouldStop: FnMut() -> bool,
    {
        if should_stop() {
            return None;
        }
        self.activate();
        if self.next_index < self.store.len() {
            let id = CandidateId::new(self.next_index);
            self.next_index += 1;
            return Some(id);
        }
        let assignment_cursor = self.assignment_cursor.as_mut()?;
        let mov = assignment_cursor
            .next_move_with_control(should_stop)?
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

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
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
        GroupedScalarCursor::deferred(GroupedScalarActivation {
            group: self.group.clone(),
            solution: solution.clone(),
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
