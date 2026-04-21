use std::fmt::Debug;
use std::hash::Hash;
use std::time::Instant;

use solverforge_config::{ConstructionHeuristicConfig, ConstructionHeuristicType};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::{Director, RecordingDirector};
use tracing::info;

use crate::builder::{
    ListVariableContext, ModelContext, ScalarVariableContext, ValueSource, VariableContext,
};
use crate::heuristic::r#move::{ChangeMove, Move};
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepScope};
use crate::stats::{format_duration, whole_units_per_second};

use super::decision::{
    is_first_fit_improvement, select_best_fit, select_first_fit, ScoredChoiceTracker,
};
use super::evaluation::evaluate_trial_move;
use super::ConstructionListElementId;
use super::ConstructionSlotId;

enum Candidate<S, V>
where
    S: PlanningSolution,
{
    Scalar {
        getter: fn(&S, usize) -> Option<usize>,
        setter: fn(&mut S, usize, Option<usize>),
        variable_name: &'static str,
        descriptor_index: usize,
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
                solve_first_fit_iteration(model, &mut phase_scope, entity_class, variable_name)
            }
            ConstructionHeuristicType::CheapestInsertion => {
                solve_best_fit_iteration(model, &mut phase_scope, entity_class, variable_name)
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

fn solve_first_fit_iteration<S, V, DM, IDM, D, ProgressCb>(
    model: &ModelContext<S, V, DM, IDM>,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    entity_class: Option<&str>,
    variable_name: Option<&str>,
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
                solve_scalar_first_fit(variable_index, *ctx, phase_scope)
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
            VariableContext::Scalar(ctx) => scan_scalar_best_fit(variable_index, *ctx, phase_scope),
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

fn solve_scalar_first_fit<S, V, D, ProgressCb>(
    variable_index: usize,
    ctx: ScalarVariableContext<S>,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) -> IterationProgress<S, V>
where
    S: PlanningSolution,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let entity_count = (ctx.entity_count)(phase_scope.score_director().working_solution());

    for entity_index in 0..entity_count {
        let slot_id = ConstructionSlotId::new(ctx.descriptor_index, entity_index);
        if phase_scope
            .solver_scope()
            .is_standard_slot_completed(slot_id)
        {
            continue;
        }

        let current = (ctx.getter)(
            phase_scope.score_director().working_solution(),
            entity_index,
        );
        if current.is_some() {
            continue;
        }

        let values = scalar_values_for_entity(
            ctx,
            phase_scope.score_director().working_solution(),
            entity_index,
        );
        if values.is_empty() {
            if ctx.allows_unassigned {
                complete_scalar_slot(slot_id, phase_scope);
                return IterationProgress::CompletedOnly;
            }
            continue;
        }

        let mut first_doable = None;
        let baseline_score = ctx.allows_unassigned.then(|| phase_scope.calculate_score());

        for (value_index, value) in values.into_iter().enumerate() {
            let mov = ChangeMove::new(
                entity_index,
                Some(value),
                ctx.getter,
                ctx.setter,
                ctx.variable_name,
                ctx.descriptor_index,
            );
            if !mov.is_doable(phase_scope.score_director()) {
                continue;
            }

            let score = candidate_score(phase_scope, &mov);

            if let Some(baseline_score) = baseline_score {
                if is_first_fit_improvement(baseline_score, score) {
                    first_doable = Some((value_index, value, score));
                    break;
                }
            } else {
                first_doable = Some((value_index, value, score));
                break;
            }
        }

        let selection = select_first_fit(
            first_doable
                .as_ref()
                .map(|(value_index, _, _)| *value_index),
        );

        match selection {
            crate::phase::construction::ConstructionChoice::Select(selected_index) => {
                let Some((value_index, value, score)) =
                    first_doable.filter(|(value_index, _, _)| *value_index == selected_index)
                else {
                    unreachable!("selected scalar construction candidate should exist");
                };
                return IterationProgress::Committed(Candidate::Scalar {
                    getter: ctx.getter,
                    setter: ctx.setter,
                    variable_name: ctx.variable_name,
                    descriptor_index: ctx.descriptor_index,
                    entity_index,
                    value,
                    order_key: [variable_index, entity_index, value_index, 0],
                    score,
                });
            }
            crate::phase::construction::ConstructionChoice::KeepCurrent
                if ctx.allows_unassigned =>
            {
                complete_scalar_slot(slot_id, phase_scope);
                return IterationProgress::CompletedOnly;
            }
            crate::phase::construction::ConstructionChoice::KeepCurrent => {}
        }
    }

    IterationProgress::None
}

fn solve_list_first_fit<S, V, DM, IDM, D, ProgressCb>(
    list_index: usize,
    ctx: ListVariableContext<S, V, DM, IDM>,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
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
    let entity_count = (ctx.entity_count)(phase_scope.score_director().working_solution());
    if entity_count == 0 {
        return IterationProgress::None;
    }

    let assigned = (ctx.assigned_elements)(phase_scope.score_director().working_solution());
    let assigned_set: std::collections::HashSet<V> = assigned.into_iter().collect();
    let element_count = (ctx.element_count)(phase_scope.score_director().working_solution());

    for element_index in 0..element_count {
        let element_id = ConstructionListElementId::new(list_index, element_index);
        if phase_scope
            .solver_scope()
            .is_list_element_completed(element_id)
        {
            continue;
        }

        let element = (ctx.index_to_element)(
            phase_scope.score_director().working_solution(),
            element_index,
        );
        if assigned_set.contains(&element) {
            continue;
        }

        let entity_index = 0;
        let position = 0;
        let score = evaluate_list_insertion(phase_scope, &ctx, element, entity_index, position);
        return IterationProgress::Committed(Candidate::List {
            list_insert: ctx.list_insert,
            descriptor_index: ctx.descriptor_index,
            element,
            entity_index,
            position,
            order_key: [list_index, element_index, entity_index, position],
            score,
        });
    }

    IterationProgress::None
}

fn scan_scalar_best_fit<S, V, D, ProgressCb>(
    variable_index: usize,
    ctx: ScalarVariableContext<S>,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) -> IterationProgress<S, V>
where
    S: PlanningSolution,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let entity_count = (ctx.entity_count)(phase_scope.score_director().working_solution());

    for entity_index in 0..entity_count {
        let slot_id = ConstructionSlotId::new(ctx.descriptor_index, entity_index);
        if phase_scope
            .solver_scope()
            .is_standard_slot_completed(slot_id)
        {
            continue;
        }

        let current = (ctx.getter)(
            phase_scope.score_director().working_solution(),
            entity_index,
        );
        if current.is_some() {
            continue;
        }

        let values = scalar_values_for_entity(
            ctx,
            phase_scope.score_director().working_solution(),
            entity_index,
        );
        if values.is_empty() {
            if ctx.allows_unassigned {
                complete_scalar_slot(slot_id, phase_scope);
                return IterationProgress::CompletedOnly;
            }
            continue;
        }

        let baseline_score = ctx.allows_unassigned.then(|| phase_scope.calculate_score());
        let mut tracker = ScoredChoiceTracker::default();
        let mut best: Option<(usize, usize, S::Score)> = None;

        for (value_index, value) in values.into_iter().enumerate() {
            let mov = ChangeMove::new(
                entity_index,
                Some(value),
                ctx.getter,
                ctx.setter,
                ctx.variable_name,
                ctx.descriptor_index,
            );
            if !mov.is_doable(phase_scope.score_director()) {
                continue;
            }
            let score = candidate_score(phase_scope, &mov);
            tracker.consider(value_index, score);
            let should_replace = match best {
                None => true,
                Some((_, _, best_score)) => score > best_score,
            };
            if should_replace {
                best = Some((value_index, value, score));
            }
        }

        match (select_best_fit(tracker, baseline_score), best) {
            (_, None) => {
                if ctx.allows_unassigned {
                    complete_scalar_slot(slot_id, phase_scope);
                    return IterationProgress::CompletedOnly;
                }
            }
            (
                crate::phase::construction::ConstructionChoice::Select(_),
                Some((value_index, value, score)),
            ) => {
                return IterationProgress::Committed(Candidate::Scalar {
                    getter: ctx.getter,
                    setter: ctx.setter,
                    variable_name: ctx.variable_name,
                    descriptor_index: ctx.descriptor_index,
                    entity_index,
                    value,
                    order_key: [variable_index, entity_index, value_index, 0],
                    score,
                });
            }
            (crate::phase::construction::ConstructionChoice::KeepCurrent, Some(_)) => {
                complete_scalar_slot(slot_id, phase_scope);
                return IterationProgress::CompletedOnly;
            }
        }
    }

    IterationProgress::None
}

fn scan_list_best_fit<S, V, DM, IDM, D, ProgressCb>(
    list_index: usize,
    ctx: ListVariableContext<S, V, DM, IDM>,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
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
    let entity_count = (ctx.entity_count)(phase_scope.score_director().working_solution());
    if entity_count == 0 {
        return IterationProgress::None;
    }

    let assigned = (ctx.assigned_elements)(phase_scope.score_director().working_solution());
    let assigned_set: std::collections::HashSet<V> = assigned.into_iter().collect();
    let element_count = (ctx.element_count)(phase_scope.score_director().working_solution());
    let mut best_candidate: Option<Candidate<S, V>> = None;

    for element_index in 0..element_count {
        let element_id = ConstructionListElementId::new(list_index, element_index);
        if phase_scope
            .solver_scope()
            .is_list_element_completed(element_id)
        {
            continue;
        }

        let element = (ctx.index_to_element)(
            phase_scope.score_director().working_solution(),
            element_index,
        );
        if assigned_set.contains(&element) {
            continue;
        }

        let mut best_for_element: Option<(usize, usize, S::Score)> = None;
        for entity_index in 0..entity_count {
            let len = (ctx.list_len)(
                phase_scope.score_director().working_solution(),
                entity_index,
            );
            for position in 0..=len {
                let score =
                    evaluate_list_insertion(phase_scope, &ctx, element, entity_index, position);
                let should_replace = match best_for_element {
                    None => true,
                    Some((_, _, best_score)) => score > best_score,
                };
                if should_replace {
                    best_for_element = Some((entity_index, position, score));
                }
            }
        }

        if let Some((entity_index, position, score)) = best_for_element {
            update_best_candidate(
                &mut best_candidate,
                Candidate::List {
                    list_insert: ctx.list_insert,
                    descriptor_index: ctx.descriptor_index,
                    element,
                    entity_index,
                    position,
                    order_key: [list_index, element_index, entity_index, position],
                    score,
                },
            );
        } else {
            phase_scope
                .solver_scope_mut()
                .mark_list_element_completed(element_id);
        }
    }

    best_candidate
        .map(IterationProgress::Committed)
        .unwrap_or(IterationProgress::None)
}

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
            entity_index,
            value,
            ..
        } => {
            let mov = ChangeMove::new(
                entity_index,
                Some(value),
                getter,
                setter,
                variable_name,
                descriptor_index,
            );
            let mut step_scope = StepScope::new(phase_scope);
            step_scope.phase_scope_mut().record_move_accepted();
            step_scope.apply_committed_move(&mov);
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
            let step_score = step_scope.calculate_score();
            step_scope.set_step_score(step_score);
            step_scope.complete();
        }
    }
}

fn complete_scalar_slot<S, D, ProgressCb>(
    slot_id: ConstructionSlotId,
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
        .mark_standard_slot_completed(slot_id);
    let step_score = step_scope.calculate_score();
    step_scope.set_step_score(step_score);
    step_scope.complete();
}

fn scalar_values_for_entity<S>(
    ctx: ScalarVariableContext<S>,
    solution: &S,
    entity_index: usize,
) -> Vec<usize> {
    match ctx.value_source {
        ValueSource::Empty => Vec::new(),
        ValueSource::CountableRange { from, to } => (from..to).collect(),
        ValueSource::SolutionCount { count_fn } => (0..count_fn(solution)).collect(),
        ValueSource::EntitySlice { values_for_entity } => {
            values_for_entity(solution, entity_index).to_vec()
        }
    }
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
