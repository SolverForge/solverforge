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
        Self {
            group,
            value_candidate_limit,
            max_moves_per_step: max_moves_per_step.unwrap_or(256),
            require_hard_improvement,
        }
    }

    fn limits(&self) -> crate::builder::context::ScalarGroupLimits {
        crate::builder::context::ScalarGroupLimits {
            value_candidate_limit: self.value_candidate_limit,
            group_candidate_limit: None,
            max_moves_per_step: Some(self.max_moves_per_step),
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
        let solution = score_director.working_solution();
        let mut store = CandidateStore::with_capacity(self.max_moves_per_step);
        let mut seen = std::collections::HashSet::new();
        let mut targets = std::collections::HashSet::new();

        for candidate in (self.group.candidate_provider)(solution, self.limits()) {
            if store.len() >= self.max_moves_per_step {
                break;
            }
            if candidate.edits().is_empty() || !seen.insert(candidate.clone()) {
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

            let Some(mov) = compound_move_for_group_candidate(&self.group, solution, candidate)
            else {
                continue;
            };
            let mov = mov.with_require_hard_improvement(self.require_hard_improvement);
            if mov.is_doable(score_director) {
                store.push(ScalarMoveUnion::CompoundScalar(mov));
            }
        }

        GroupedScalarCursor::new(store)
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, _score_director: &D) -> usize {
        self.max_moves_per_step
    }
}

fn compound_move_for_group_candidate<S>(
    group: &crate::builder::context::ScalarGroupBinding<S>,
    solution: &S,
    candidate: crate::builder::context::ScalarCandidate<S>,
) -> Option<crate::heuristic::r#move::CompoundScalarMove<S>>
where
    S: PlanningSolution + 'static,
{
    let reason = candidate.reason();
    let mut edits = Vec::with_capacity(candidate.edits().len());
    for edit in candidate.into_edits() {
        let member = group.member_for_edit(&edit)?;
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
