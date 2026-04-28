enum Candidate<S, V>
where
    S: PlanningSolution,
{
    Scalar {
        getter: ScalarGetter<S>,
        setter: ScalarSetter<S>,
        variable_name: &'static str,
        descriptor_index: usize,
        variable_index: usize,
        entity_index: usize,
        value: usize,
        order_key: [usize; 4],
        score: S::Score,
    },
    List {
        list_insert: fn(&mut S, usize, usize, V),
        descriptor_index: usize,
        element: V,
        entity_index: usize,
        position: usize,
        order_key: [usize; 4],
        score: S::Score,
    },
}

impl<S, V> Candidate<S, V>
where
    S: PlanningSolution,
{
    fn score(&self) -> &S::Score {
        match self {
            Self::Scalar { score, .. } | Self::List { score, .. } => score,
        }
    }

    fn order_key(&self) -> &[usize; 4] {
        match self {
            Self::Scalar { order_key, .. } | Self::List { order_key, .. } => order_key,
        }
    }
}

enum IterationProgress<S, V>
where
    S: PlanningSolution,
{
    None,
    CompletedOnly,
    Committed(Candidate<S, V>),
}

pub(crate) fn solve_construction<S, V, DM, IDM, D, ProgressCb>(
    config: Option<&ConstructionHeuristicConfig>,
    model: &ModelContext<S, V, DM, IDM>,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> bool
where
    S: PlanningSolution,
    S::Score: Score + Copy,
    V: Clone + Copy + PartialEq + Eq + Hash + Send + Sync + Debug + 'static,
    DM: Clone + Debug + 'static,
    IDM: Clone + Debug + 'static,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let entity_class = config.and_then(|cfg| cfg.target.entity_class.as_deref());
    let variable_name = config.and_then(|cfg| cfg.target.variable_name.as_deref());
    let explicit_target = entity_class.is_some() || variable_name.is_some();
    let has_match = model
        .variables()
        .iter()
        .any(|variable| matches_target(variable, entity_class, variable_name));

    if explicit_target && !has_match {
        panic!(
            "construction heuristic matched no planning variables for entity_class={:?} variable_name={:?}",
            entity_class,
            variable_name
        );
    }

    let heuristic = config
        .map(|cfg| cfg.construction_heuristic_type)
        .unwrap_or(ConstructionHeuristicType::FirstFit);
    let value_candidate_limit = config.and_then(|cfg| cfg.value_candidate_limit);
    if heuristic == ConstructionHeuristicType::CheapestInsertion {
        let unbounded = model.variables().iter().any(|variable| {
            matches_target(variable, entity_class, variable_name)
                && matches!(
                    variable,
                    VariableContext::Scalar(ctx)
                        if ctx.candidate_values.is_none() && value_candidate_limit.is_none()
                )
        });
        assert!(
            !unbounded,
            "cheapest_insertion scalar construction requires candidate_values or value_candidate_limit",
        );
    }

    let mut phase_scope = PhaseScope::with_phase_type(solver_scope, 0, "Construction Heuristic");
    let phase_index = phase_scope.phase_index();
    let previous_best_score = phase_scope.solver_scope().best_score().copied();
    let mut ran_step = false;

    info!(
        event = "phase_start",
        phase = "Construction Heuristic",
        phase_index = phase_index,
    );

    loop {
        if phase_scope
            .solver_scope_mut()
            .should_terminate_construction()
        {
            break;
        }

        let progress = match heuristic {
            ConstructionHeuristicType::FirstFit => {
                solve_first_fit_iteration(
                    model,
                    &mut phase_scope,
                    entity_class,
                    variable_name,
                    value_candidate_limit,
                )
            }
            ConstructionHeuristicType::CheapestInsertion => {
                solve_best_fit_iteration(
                    model,
                    &mut phase_scope,
                    entity_class,
                    variable_name,
                    value_candidate_limit,
                )
            }
            other => panic!("unsupported generic construction heuristic {other:?}"),
        };

        match progress {
            IterationProgress::None => break,
            IterationProgress::CompletedOnly => {
                ran_step = true;
                continue;
            }
            IterationProgress::Committed(candidate) => {
                ran_step = true;
                commit_candidate(candidate, &mut phase_scope);
            }
        }
    }

    if ran_step {
        phase_scope.update_best_solution();
        if phase_scope.solver_scope().current_score() == previous_best_score.as_ref() {
            phase_scope.promote_current_solution_on_score_tie();
        }
    }

    let best_score = phase_scope
        .solver_scope()
        .best_score()
        .map(|s| format!("{}", s))
        .unwrap_or_else(|| "none".to_string());
    let duration = phase_scope.elapsed();
    let steps = phase_scope.step_count();
    let speed = whole_units_per_second(steps, duration);
    let stats = phase_scope.stats();

    info!(
        event = "phase_end",
        phase = "Construction Heuristic",
        phase_index = phase_index,
        duration = %format_duration(duration),
        steps = steps,
        moves_generated = stats.moves_generated,
        moves_evaluated = stats.moves_evaluated,
        moves_accepted = stats.moves_accepted,
        score_calculations = stats.score_calculations,
        generation_time = %format_duration(stats.generation_time()),
        evaluation_time = %format_duration(stats.evaluation_time()),
        speed = speed,
        score = best_score,
    );

    ran_step
}
