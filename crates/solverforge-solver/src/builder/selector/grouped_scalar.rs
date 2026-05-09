pub struct GroupedScalarSelector<S> {
    group: crate::builder::context::ScalarGroupBinding<S>,
    value_candidate_limit: Option<usize>,
    max_moves_per_step: usize,
    require_hard_improvement: bool,
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
        }
    }

    fn limits(&self) -> crate::builder::context::ScalarGroupLimits {
        crate::builder::context::ScalarGroupLimits {
            value_candidate_limit: self.value_candidate_limit,
            group_candidate_limit: None,
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
        let candidate_provider = match self.group.kind {
            crate::builder::ScalarGroupBindingKind::Candidates { candidate_provider } => {
                candidate_provider
            }
            crate::builder::ScalarGroupBindingKind::Assignment(assignment) => {
                return self.open_assignment_cursor(score_director, assignment);
            }
        };
        let solution = score_director.working_solution();
        let mut store = CandidateStore::with_capacity(self.max_moves_per_step);
        let mut seen_candidates = Vec::new();
        let mut targets = std::collections::HashSet::new();

        for candidate in candidate_provider(solution, self.limits()) {
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

            let Some(mov) = compound_move_for_group_candidate(&self.group, solution, &candidate)
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
    ) -> GroupedScalarCursor<S> {
        let solution = score_director.working_solution();
        let options = crate::phase::construction::grouped_scalar::ScalarAssignmentMoveOptions::for_selector(
            self.group.limits,
            self.value_candidate_limit,
            self.max_moves_per_step,
        );
        let mut store = CandidateStore::with_capacity(self.max_moves_per_step);
        for mov in crate::phase::construction::grouped_scalar::required_assignment_moves(
            &assignment,
            solution,
            options,
        )
        .into_iter()
        .chain(crate::phase::construction::grouped_scalar::capacity_conflict_moves(
            &assignment,
            solution,
            options,
        ))
        .chain(crate::phase::construction::grouped_scalar::reassignment_moves(
            &assignment,
            solution,
            options,
        ))
        .chain(crate::phase::construction::grouped_scalar::rematch_assignment_moves(
            &assignment,
            solution,
            options,
        )) {
            if store.len() >= self.max_moves_per_step {
                break;
            }
            let mov = mov.with_require_hard_improvement(self.require_hard_improvement);
            if mov.is_doable(score_director) {
                store.push(ScalarMoveUnion::CompoundScalar(mov));
            }
        }
        GroupedScalarCursor::new(store)
    }
}

fn compound_move_for_group_candidate<S>(
    group: &crate::builder::context::ScalarGroupBinding<S>,
    solution: &S,
    candidate: &crate::builder::context::ScalarCandidate<S>,
) -> Option<crate::heuristic::r#move::CompoundScalarMove<S>>
where
    S: PlanningSolution + 'static,
{
    let reason = candidate.reason();
    let mut edits = Vec::with_capacity(candidate.edits().len());
    for edit in candidate.edits() {
        let member = group.member_for_edit(edit)?;
        if !member.value_is_legal(solution, edit.entity_index(), edit.to_value()) {
            return None;
        }
        edits.push(crate::heuristic::r#move::CompoundScalarEdit {
            descriptor_index: member.descriptor_index,
            entity_index: edit.entity_index(),
            variable_index: member.variable_index,
            variable_name: member.variable_name,
            to_value: edit.to_value(),
            getter: member.getter,
            setter: member.setter,
            value_is_legal: None,
        });
    }

    Some(crate::heuristic::r#move::CompoundScalarMove::with_label(
        reason,
        crate::heuristic::r#move::COMPOUND_SCALAR_VARIABLE,
        edits,
    ))
}
