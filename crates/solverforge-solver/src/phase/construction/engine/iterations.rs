fn solve_first_fit_iteration<S, V, DM, IDM, D, ProgressCb>(
    model: &ModelContext<S, V, DM, IDM>,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    entity_class: Option<&str>,
    variable_name: Option<&str>,
    value_candidate_limit: Option<usize>,
    construction_obligation: ConstructionObligation,
) -> IterationProgress<S, V>
where
    S: PlanningSolution,
    S::Score: Score + Copy,
    V: Clone + Copy + PartialEq + Eq + Hash + Send + Sync + Debug + 'static,
    DM: Clone + Debug + 'static,
    IDM: Clone + Debug + 'static,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let mut completed_only = false;

    for (variable_index, variable) in model.variables().iter().enumerate() {
        if !matches_target(variable, entity_class, variable_name) {
            continue;
        }

        let progress = match variable {
            VariableContext::Scalar(ctx) => {
                solve_scalar_first_fit(
                    variable_index,
                    *ctx,
                    value_candidate_limit,
                    construction_obligation,
                    phase_scope,
                )
            }
            VariableContext::List(ctx) => {
                solve_list_first_fit(variable_index, ctx.clone(), phase_scope)
            }
        };

        match progress {
            IterationProgress::None => {}
            IterationProgress::CompletedOnly => completed_only = true,
            IterationProgress::Committed(candidate) => {
                return IterationProgress::Committed(candidate);
            }
        }
    }

    if completed_only {
        IterationProgress::CompletedOnly
    } else {
        IterationProgress::None
    }
}

fn solve_best_fit_iteration<S, V, DM, IDM, D, ProgressCb>(
    model: &ModelContext<S, V, DM, IDM>,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    entity_class: Option<&str>,
    variable_name: Option<&str>,
    value_candidate_limit: Option<usize>,
    construction_obligation: ConstructionObligation,
) -> IterationProgress<S, V>
where
    S: PlanningSolution,
    S::Score: Score + Copy,
    V: Clone + Copy + PartialEq + Eq + Hash + Send + Sync + Debug + 'static,
    DM: Clone + Debug + 'static,
    IDM: Clone + Debug + 'static,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let mut best_candidate: Option<Candidate<S, V>> = None;
    let mut completed_only = false;

    for (variable_index, variable) in model.variables().iter().enumerate() {
        if !matches_target(variable, entity_class, variable_name) {
            continue;
        }

        let progress = match variable {
            VariableContext::Scalar(ctx) => {
                scan_scalar_best_fit(
                    variable_index,
                    *ctx,
                    value_candidate_limit,
                    construction_obligation,
                    phase_scope,
                )
            }
            VariableContext::List(ctx) => {
                scan_list_best_fit(variable_index, ctx.clone(), phase_scope)
            }
        };

        match progress {
            IterationProgress::None => {}
            IterationProgress::CompletedOnly => completed_only = true,
            IterationProgress::Committed(candidate) => {
                update_best_candidate(&mut best_candidate, candidate);
            }
        }
    }

    if let Some(candidate) = best_candidate {
        IterationProgress::Committed(candidate)
    } else if completed_only {
        IterationProgress::CompletedOnly
    } else {
        IterationProgress::None
    }
}
