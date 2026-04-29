fn commit_candidate<S, V, D, ProgressCb>(
    candidate: Candidate<S, V>,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) where
    S: PlanningSolution,
    S::Score: Score + Copy,
    V: Clone + Copy + PartialEq + Eq + Hash + Send + Sync + Debug + 'static,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    match candidate {
        Candidate::Scalar {
            getter,
            setter,
            variable_name,
            descriptor_index,
            variable_index,
            entity_index,
            value,
            ..
        } => {
            let mov = ChangeMove::new(
                entity_index,
                Some(value),
                getter,
                setter,
                variable_index,
                variable_name,
                descriptor_index,
            );
            let mut step_scope = StepScope::new(phase_scope);
            step_scope.phase_scope_mut().record_move_accepted();
            step_scope.apply_committed_move(&mov);
            step_scope.phase_scope_mut().record_move_applied();
            step_scope
                .phase_scope_mut()
                .record_construction_slot_assigned();
            let step_score = step_scope.calculate_score();
            step_scope.set_step_score(step_score);
            step_scope.complete();
        }
        Candidate::List {
            list_insert,
            descriptor_index,
            element,
            entity_index,
            position,
            ..
        } => {
            let mut step_scope = StepScope::new(phase_scope);
            step_scope.phase_scope_mut().record_move_accepted();
            step_scope.apply_committed_change(|score_director| {
                score_director.before_variable_changed(descriptor_index, entity_index);
                list_insert(
                    score_director.working_solution_mut(),
                    entity_index,
                    position,
                    element,
                );
                score_director.after_variable_changed(descriptor_index, entity_index);
            });
            step_scope.phase_scope_mut().record_move_applied();
            let step_score = step_scope.calculate_score();
            step_scope.set_step_score(step_score);
            step_scope.complete();
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ScalarSlotCompletion {
    Kept,
    NoDoableCandidate,
}

fn complete_scalar_slot<S, D, ProgressCb>(
    slot_id: ConstructionSlotId,
    completion: ScalarSlotCompletion,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) where
    S: PlanningSolution,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let mut step_scope = StepScope::new(phase_scope);
    step_scope
        .phase_scope_mut()
        .solver_scope_mut()
        .mark_scalar_slot_completed(slot_id);
    match completion {
        ScalarSlotCompletion::Kept => step_scope
            .phase_scope_mut()
            .record_construction_slot_kept(),
        ScalarSlotCompletion::NoDoableCandidate => step_scope
            .phase_scope_mut()
            .record_construction_slot_no_doable(),
    }
    let step_score = step_scope.calculate_score();
    step_scope.set_step_score(step_score);
    step_scope.complete();
}

fn evaluate_list_insertion<S, V, DM, IDM, D, ProgressCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    ctx: &ListVariableContext<S, V, DM, IDM>,
    element: V,
    entity_index: usize,
    position: usize,
) -> S::Score
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    V: Copy + 'static,
{
    let generation_started = Instant::now();
    phase_scope.record_generated_move(generation_started.elapsed());

    let mut recording = RecordingDirector::new(phase_scope.score_director_mut());
    let evaluation_started = Instant::now();
    recording.before_variable_changed(ctx.descriptor_index, entity_index);
    (ctx.list_insert)(
        recording.working_solution_mut(),
        entity_index,
        position,
        element,
    );
    recording.after_variable_changed(ctx.descriptor_index, entity_index);
    let remove = ctx.construction_list_remove;
    recording.register_undo(Box::new(move |solution: &mut S| {
        remove(solution, entity_index, position);
    }));
    let score = recording.calculate_score();
    recording.undo_changes();
    phase_scope.record_score_calculation();
    phase_scope.record_evaluated_move(evaluation_started.elapsed());
    score
}

fn candidate_score<S, D, M, ProgressCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    mov: &M,
) -> S::Score
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    M: Move<S>,
{
    let generation_started = Instant::now();
    phase_scope.record_generated_move(generation_started.elapsed());

    let evaluation_started = Instant::now();
    let score = evaluate_trial_move(phase_scope.score_director_mut(), mov);
    phase_scope.record_score_calculation();
    phase_scope.record_evaluated_move(evaluation_started.elapsed());
    score
}

fn matches_target<S, V, DM, IDM>(
    variable: &VariableContext<S, V, DM, IDM>,
    entity_class: Option<&str>,
    variable_name: Option<&str>,
) -> bool {
    match variable {
        VariableContext::Scalar(ctx) => ctx.matches_target(entity_class, variable_name),
        VariableContext::List(ctx) => ctx.matches_target(entity_class, variable_name),
    }
}

fn update_best_candidate<S, V>(slot: &mut Option<Candidate<S, V>>, candidate: Candidate<S, V>)
where
    S: PlanningSolution,
{
    let should_replace = match slot {
        None => true,
        Some(current) => {
            candidate.score() > current.score()
                || (candidate.score() == current.score()
                    && candidate.order_key() < current.order_key())
        }
    };

    if should_replace {
        *slot = Some(candidate);
    }
}
