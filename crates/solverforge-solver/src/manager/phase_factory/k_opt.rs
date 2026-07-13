/* K-opt phase builder for tour optimization.

Creates a local search phase that uses k-opt moves to improve solutions.
This is commonly used for vehicle routing and traveling salesman problems.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::selector::k_opt::{KOptConfig, KOptMoveSelector};
use crate::heuristic::selector::move_selector::MoveCursor;
use crate::phase::control::{
    settle_search_interrupt, should_interrupt_evaluation, StepInterrupt, GENERATION_POLL_INTERVAL,
};
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};
use crate::stats::{CandidateTraceDisposition, CandidateTracePullToken, CandidateTraceSource};

use super::super::PhaseFactory;

/// Builder for creating k-opt local search phases.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `V` - The list element value type (e.g., `usize` for visit indices)
/// # Example
///
/// ```
/// use solverforge_solver::KOptPhaseBuilder;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone, Debug)]
/// struct Vehicle { visits: Vec<usize> }
///
/// #[derive(Clone, Debug)]
/// struct Plan {
///     vehicles: Vec<Vehicle>,
///     score: Option<SoftScore>,
/// }
///
/// impl PlanningSolution for Plan {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn list_len(s: &Plan, idx: usize) -> usize {
///     s.vehicles.get(idx).map_or(0, |v| v.visits.len())
/// }
///
/// fn sublist_remove(s: &mut Plan, idx: usize, start: usize, end: usize) -> Vec<usize> {
///     s.vehicles.get_mut(idx)
///         .map(|v| v.visits.drain(start..end).collect())
///         .unwrap_or_default()
/// }
///
/// fn sublist_insert(s: &mut Plan, idx: usize, pos: usize, items: Vec<usize>) {
///     if let Some(v) = s.vehicles.get_mut(idx) {
///         for (i, item) in items.into_iter().enumerate() {
///             v.visits.insert(pos + i, item);
///         }
///     }
/// }
///
/// let builder = KOptPhaseBuilder::<Plan, usize>::new(
///     list_len,
///     |s, idx, pos| s.vehicles.get(idx).and_then(|v| v.visits.get(pos)).copied(),
///     sublist_remove,
///     sublist_insert,
///     "visits",
///     0,
/// );
/// ```
pub struct KOptPhaseBuilder<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    k: usize,
    step_limit: Option<u64>,
    _marker: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V> KOptPhaseBuilder<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    /// Creates a new k-opt phase builder.
    pub fn new(
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            k: 3, // Default to 3-opt
            step_limit: Some(1000),
            _marker: PhantomData,
        }
    }

    pub fn with_k(mut self, k: usize) -> Self {
        assert!((2..=5).contains(&k), "k must be between 2 and 5");
        self.k = k;
        self
    }

    pub fn with_step_limit(mut self, limit: u64) -> Self {
        self.step_limit = Some(limit);
        self
    }

    /// Removes the step limit.
    pub fn without_step_limit(mut self) -> Self {
        self.step_limit = None;
        self
    }
}

impl<S, V> Debug for KOptPhaseBuilder<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KOptPhaseBuilder")
            .field("k", &self.k)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .field("step_limit", &self.step_limit)
            .finish()
    }
}

/// K-opt local search phase.
///
/// Iterates through k-opt moves and accepts improving ones using hill climbing.
pub struct KOptPhase<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    config: KOptConfig,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    step_limit: Option<u64>,
    _marker: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V> Debug for KOptPhase<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KOptPhase")
            .field("k", &self.config.k)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V, D> Phase<S, D> for KOptPhase<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    D: Director<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        use crate::heuristic::r#move::Move;
        use crate::heuristic::selector::entity::FromSolutionEntitySelector;
        use crate::heuristic::selector::move_selector::MoveSelector;

        let mut phase_scope = PhaseScope::with_phase_type(solver_scope, 0, "K-Opt");

        // Calculate initial score
        let mut last_step_score = phase_scope.calculate_score();

        // Create move selector
        let entity_selector = FromSolutionEntitySelector::new(self.descriptor_index);
        let move_selector = KOptMoveSelector::<S, V, _>::new(
            entity_selector,
            self.config.clone(),
            self.list_len,
            self.list_get,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.descriptor_index,
        );

        let step_limit = self.step_limit.unwrap_or(u64::MAX);
        let mut steps = 0u64;
        while steps < step_limit && !phase_scope.solver_scope_mut().should_terminate() {
            let mut step_scope = StepScope::new(&mut phase_scope);
            let mut cursor = move_selector.open_cursor(step_scope.score_director());
            let mut best_move_id = None;
            let mut best_score = None;
            let mut best_trace_token: Option<CandidateTracePullToken> = None;
            let mut interrupted_step = false;
            let mut candidate_ordinal = 0usize;

            while let Some(candidate_id) = cursor.next_candidate() {
                let candidate = cursor
                    .candidate(candidate_id)
                    .expect("k-opt candidate id must remain live after pull");
                let trace_token = step_scope.phase_scope_mut().record_candidate_pull(
                    CandidateTraceSource::KOpt,
                    None,
                    candidate_id.index(),
                    None,
                    &candidate,
                );
                if should_interrupt_evaluation(&step_scope, candidate_ordinal) {
                    if let Some(token) = trace_token {
                        step_scope
                            .phase_scope_mut()
                            .record_candidate_trace_disposition(
                                token,
                                CandidateTraceDisposition::InterruptedBeforeEvaluation,
                            );
                    }
                    interrupted_step = true;
                    break;
                }
                candidate_ordinal += 1;

                let is_doable = cursor
                    .candidate(candidate_id)
                    .expect("k-opt candidate id must remain live during evaluation")
                    .is_doable(step_scope.score_director());
                if !is_doable {
                    if let Some(token) = trace_token {
                        let phase_scope = step_scope.phase_scope_mut();
                        phase_scope.record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::Evaluated,
                        );
                        phase_scope.record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::NotDoable,
                        );
                    }
                    assert!(cursor.release_candidate(candidate_id));
                    if candidate_ordinal.is_multiple_of(GENERATION_POLL_INTERVAL) {
                        step_scope.phase_scope_mut().report_progress_if_due();
                    }
                    continue;
                }

                let move_score = {
                    let mv = cursor
                        .candidate(candidate_id)
                        .expect("k-opt candidate id must remain live during evaluation");
                    let score_state = step_scope.score_director().snapshot_score_state();
                    let undo = mv.do_move(step_scope.score_director_mut());
                    let move_score = step_scope.calculate_score();
                    mv.undo_move(step_scope.score_director_mut(), undo);
                    step_scope
                        .score_director_mut()
                        .restore_score_state(score_state);
                    move_score
                };
                if let Some(token) = trace_token {
                    step_scope
                        .phase_scope_mut()
                        .record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::Evaluated,
                        );
                }

                if move_score > last_step_score
                    && best_score.as_ref().is_none_or(|b| move_score > *b)
                {
                    if let Some(previous_best) = best_move_id.replace(candidate_id) {
                        assert!(cursor.release_candidate(previous_best));
                    }
                    if let Some(token) = std::mem::replace(&mut best_trace_token, trace_token) {
                        step_scope
                            .phase_scope_mut()
                            .record_candidate_trace_disposition(
                                token,
                                CandidateTraceDisposition::ForagerIgnored,
                            );
                    }
                    best_score = Some(move_score);
                } else {
                    assert!(cursor.release_candidate(candidate_id));
                    if let Some(token) = trace_token {
                        step_scope
                            .phase_scope_mut()
                            .record_candidate_trace_disposition(
                                token,
                                CandidateTraceDisposition::ForagerIgnored,
                            );
                    }
                }
                if candidate_ordinal.is_multiple_of(GENERATION_POLL_INTERVAL) {
                    step_scope.phase_scope_mut().report_progress_if_due();
                }
            }

            if interrupted_step {
                match settle_search_interrupt(&mut step_scope) {
                    StepInterrupt::Restart => {
                        if let Some(token) = best_trace_token.take() {
                            step_scope
                                .phase_scope_mut()
                                .record_candidate_trace_disposition(
                                    token,
                                    CandidateTraceDisposition::ForagerIgnored,
                                );
                        }
                        continue;
                    }
                    StepInterrupt::TerminatePhase => {
                        if let Some(token) = best_trace_token.take() {
                            step_scope
                                .phase_scope_mut()
                                .record_candidate_trace_disposition(
                                    token,
                                    CandidateTraceDisposition::ForagerIgnored,
                                );
                        }
                        break;
                    }
                }
            }

            // Apply best move if found
            if let (Some(selected_id), Some(score)) = (best_move_id, best_score) {
                if let Some(token) = best_trace_token {
                    step_scope
                        .phase_scope_mut()
                        .record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::Selected,
                        );
                }
                step_scope.apply_committed_change(|score_director| {
                    cursor.apply_owned_candidate(selected_id, score_director);
                });
                if let Some(token) = best_trace_token {
                    step_scope
                        .phase_scope_mut()
                        .record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::Applied,
                        );
                }
                step_scope.set_step_score(score);
                last_step_score = score;
                step_scope.phase_scope_mut().update_best_solution();
            } else {
                // No improving moves found - phase is done
                break;
            }

            step_scope.complete();
            steps += 1;
        }

        // Ensure we have a best solution
        if phase_scope.solver_scope().best_solution().is_none() {
            phase_scope.update_best_solution();
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "KOpt"
    }
}

impl<S, V, D> PhaseFactory<S, D> for KOptPhaseBuilder<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    D: Director<S>,
{
    type Phase = KOptPhase<S, V>;

    fn create(&self) -> Self::Phase {
        KOptPhase {
            config: KOptConfig::new(self.k),
            list_len: self.list_len,
            list_get: self.list_get,
            sublist_remove: self.sublist_remove,
            sublist_insert: self.sublist_insert,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            step_limit: self.step_limit,
            _marker: PhantomData,
        }
    }
}
