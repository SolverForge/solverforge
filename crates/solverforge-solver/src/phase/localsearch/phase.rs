// Local-search phase definition and outer solve lifecycle.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;
use tracing::info;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{CandidateId, MoveCandidateRef};
use crate::phase::localsearch::{
    Acceptor, LocalSearchForager, MoveCursorSource, SelectorCursorSource,
};
use crate::phase::Phase;
use crate::scope::{PhaseScope, ProgressCallback, SolverScope};
use crate::stats::{format_duration, whole_units_per_second, CandidateTracePullToken};

mod candidates;
mod step;

use step::{execute_step, StepOutcome};

const STEP_ACCEPTED_LABEL_LIMIT: usize = 32;

#[derive(Clone, Copy)]
struct StepMoveLabelCount {
    label: &'static str,
    count: u64,
}

struct StepMoveLabelCounts {
    entries: [StepMoveLabelCount; STEP_ACCEPTED_LABEL_LIMIT],
    overflow: u64,
}

impl StepMoveLabelCounts {
    const EMPTY_ENTRY: StepMoveLabelCount = StepMoveLabelCount {
        label: "",
        count: 0,
    };

    fn new() -> Self {
        Self {
            entries: [Self::EMPTY_ENTRY; STEP_ACCEPTED_LABEL_LIMIT],
            overflow: 0,
        }
    }

    fn record(&mut self, label: &'static str) {
        for entry in &mut self.entries {
            if entry.count > 0 && entry.label == label {
                entry.count += 1;
                return;
            }
        }
        for entry in &mut self.entries {
            if entry.count == 0 {
                entry.label = label;
                entry.count = 1;
                return;
            }
        }
        self.overflow += 1;
    }

    fn for_each_ignored_except_selected(
        &self,
        selected_label: Option<&'static str>,
        mut visitor: impl FnMut(&'static str, u64),
    ) {
        let mut selected_remaining = selected_label;
        for entry in &self.entries {
            if entry.count == 0 {
                continue;
            }
            let ignored = if selected_remaining == Some(entry.label) {
                selected_remaining = None;
                entry.count.saturating_sub(1)
            } else {
                entry.count
            };
            if ignored > 0 {
                visitor(entry.label, ignored);
            }
        }
        if self.overflow > 0 {
            visitor("move", self.overflow);
        }
    }
}

fn take_trace_token(
    tokens: &mut Vec<(CandidateId, CandidateTracePullToken)>,
    candidate_id: CandidateId,
) -> Option<CandidateTracePullToken> {
    tokens
        .iter()
        .position(|(recorded_id, _)| *recorded_id == candidate_id)
        .map(|index| tokens.swap_remove(index).1)
}

/// Local search phase that improves an existing solution.
///
/// This phase iteratively:
/// 1. Streams candidate moves through a cursor-owned store
/// 2. Evaluates each move by stable candidate ID
/// 3. Accepts/rejects based on the acceptor
/// 4. Releases losers immediately and transfers ownership of the selected move
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
/// * `Source` - The phase cursor-source type
/// * `A` - The acceptor type
/// * `Fo` - The forager type
///
/// # Zero-Clone Design
///
/// Uses ID-based online foraging. Stock foragers retain only the current winner's
/// `CandidateId` and score. The cursor releases rejected and replaced candidates,
/// then transfers the selected move by value without cloning it.
pub struct LocalSearchPhase<S, M, Source, A, Fo>
where
    S: PlanningSolution,
    M: Move<S>,
    Source: MoveCursorSource<S, M>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    move_source: Source,
    resources: Source::Resources,
    acceptor: A,
    forager: Fo,
    step_limit: Option<u64>,
    _phantom: PhantomData<fn() -> (S, M)>,
}

fn candidate_selector_label<S, M>(mov: &MoveCandidateRef<'_, S, M>) -> String
where
    S: PlanningSolution,
    M: Move<S>,
{
    let move_label = mov.telemetry_label();
    if mov.variable_name() == "compound_scalar" || mov.variable_name() == "conflict_repair" {
        return format!("{}:{move_label}", mov.variable_name());
    }
    let mut label = None;
    mov.for_each_affected_entity(&mut |affected| {
        if label.is_none() {
            label = Some(affected.variable_name.to_string());
        }
    });
    label
        .map(|variable| format!("{variable}:{move_label}"))
        .unwrap_or_else(|| format!("move:{move_label}"))
}

impl<S, M, Source, A, Fo> LocalSearchPhase<S, M, Source, A, Fo>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Source: MoveCursorSource<S, M>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    /// Builds a phase from an already-owned execution source.
    ///
    /// Configured selector compositions use this path so their stream state
    /// survives every cursor open. Ordinary public selector construction uses
    /// [`Self::new`] and reaches the same phase loop through the one
    /// `SelectorCursorSource` adapter.
    pub(crate) fn with_cursor_source(
        move_source: Source,
        resources: Source::Resources,
        acceptor: A,
        forager: Fo,
        step_limit: Option<u64>,
    ) -> Self {
        Self {
            move_source,
            resources,
            acceptor,
            forager,
            step_limit,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, MS, A, Fo> LocalSearchPhase<S, M, SelectorCursorSource<MS>, A, Fo>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    MS: crate::heuristic::selector::MoveSelector<S, M>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    /// Builds a local-search phase from an ordinary move selector.
    ///
    /// The selector is adapted to the shared phase cursor-source contract;
    /// no second local-search loop or cursor-opening path is introduced.
    pub fn new(move_selector: MS, acceptor: A, forager: Fo, step_limit: Option<u64>) -> Self {
        Self::with_cursor_source(
            SelectorCursorSource::new(move_selector),
            (),
            acceptor,
            forager,
            step_limit,
        )
    }
}

impl<S, M, Source, A, Fo> Debug for LocalSearchPhase<S, M, Source, A, Fo>
where
    S: PlanningSolution,
    M: Move<S>,
    Source: MoveCursorSource<S, M> + Debug,
    A: Acceptor<S> + Debug,
    Fo: LocalSearchForager<S, M> + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalSearchPhase")
            .field("move_source", &self.move_source)
            .field("acceptor", &self.acceptor)
            .field("forager", &self.forager)
            .field("step_limit", &self.step_limit)
            .finish()
    }
}

/// Executes the one local-search loop while borrowing its resource only for
/// cursor opening and individual candidate pulls. Both stock phases and the
/// compiled runtime call this function; neither owns a second loop.
#[allow(clippy::too_many_arguments)]
pub(crate) fn solve_local_search_with_resources<S, D, BestCb, M, Source, A, Fo>(
    move_source: &mut Source,
    resources: &mut Source::Resources,
    acceptor: &mut A,
    forager: &mut Fo,
    step_limit: Option<u64>,
    solver_scope: &mut SolverScope<S, D, BestCb>,
) where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    Source: MoveCursorSource<S, M> + Debug + Send,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    let mut phase_scope = PhaseScope::with_phase_type(solver_scope, 0, "Local Search");
    let phase_index = phase_scope.phase_index();
    let mut last_step_score = phase_scope.calculate_score();

    info!(
        event = "phase_start",
        phase = "Local Search",
        phase_index = phase_index,
        score = %last_step_score,
    );
    acceptor.phase_started(&last_step_score);

    let start_time = Instant::now();
    loop {
        if phase_scope.solver_scope_mut().should_terminate() {
            break;
        }
        if step_limit.is_some_and(|limit| phase_scope.step_count() >= limit) {
            break;
        }

        match execute_step(
            move_source,
            resources,
            acceptor,
            forager,
            &mut phase_scope,
            &mut last_step_score,
        ) {
            StepOutcome::Continue | StepOutcome::Restart => continue,
            StepOutcome::Terminate => break,
        }
    }

    acceptor.phase_ended();

    let duration = start_time.elapsed();
    let steps = phase_scope.step_count();
    let stats = phase_scope.stats();
    let speed = whole_units_per_second(stats.moves_evaluated, duration);
    let acceptance_rate = stats.acceptance_rate() * 100.0;
    let calc_speed = whole_units_per_second(stats.score_calculations, duration);
    let best_score_str = phase_scope
        .solver_scope()
        .best_score()
        .map(|s| format!("{s}"))
        .unwrap_or_else(|| "none".to_string());

    info!(
        event = "phase_end",
        phase = "Local Search",
        phase_index = phase_index,
        duration = %format_duration(duration),
        steps = steps,
        moves_generated = stats.moves_generated,
        moves_evaluated = stats.moves_evaluated,
        moves_accepted = stats.moves_accepted,
        moves_score_improving = stats.moves_score_improving(),
        moves_applied_improving = stats.moves_applied_improving(),
        score_calculations = stats.score_calculations,
        generation_time = %format_duration(stats.generation_time()),
        evaluation_time = %format_duration(stats.evaluation_time()),
        moves_speed = speed,
        calc_speed = calc_speed,
        acceptance_rate = format!("{acceptance_rate:.1}%"),
        score = best_score_str,
    );
}

impl<S, D, BestCb, M, Source, A, Fo> Phase<S, D, BestCb> for LocalSearchPhase<S, M, Source, A, Fo>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    Source: MoveCursorSource<S, M> + Debug + Send,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
        let Self {
            move_source,
            resources,
            acceptor,
            forager,
            step_limit,
            ..
        } = self;
        solve_local_search_with_resources(
            move_source,
            resources,
            acceptor,
            forager,
            *step_limit,
            solver_scope,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "LocalSearch"
    }
}

#[cfg(test)]
mod tests;
