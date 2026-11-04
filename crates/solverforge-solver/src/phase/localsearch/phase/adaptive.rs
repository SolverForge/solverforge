//! The `auto` evaluation strategy: each step runs sequentially or in parallel
//! windows, as the phase's [`AutoPolicy`] decides from measured costs.

use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveCandidateRef, ResourceMoveCursor};
use crate::phase::localsearch::parallel::{AutoPolicy, EvaluatorPool};
use crate::phase::localsearch::{Acceptor, LocalSearchForager};
use crate::scope::{ProgressCallback, SolverScope, StepScope};

use super::candidates::{evaluate_candidate_windows, CandidateLoopState};
use super::evaluator::StepEvaluator;
use super::sequential;

/// An evaluator pool plus the policy that decides, step by step, whether to
/// use it. Leaving parallel mode releases the replicas; returning rebuilds them
/// at the committed solution, and the policy accounts for that rebuild.
pub(crate) struct AdaptiveEvaluator<S, D, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pool: EvaluatorPool<S, D, M>,
    policy: AutoPolicy,
    parallel_step: bool,
}

impl<S, D, M> AdaptiveEvaluator<S, D, M>
where
    S: PlanningSolution,
    D: Director<S>,
    M: Move<S>,
{
    /// Up to `evaluators` evaluators (main director included), starting
    /// sequential, with a window overhead calibrated on the current pool.
    pub(crate) fn new(evaluators: usize) -> Self {
        Self::with_policy(evaluators, AutoPolicy::calibrate(evaluators))
    }

    pub(crate) fn with_policy(evaluators: usize, policy: AutoPolicy) -> Self {
        Self {
            pool: EvaluatorPool::new(evaluators),
            policy,
            parallel_step: false,
        }
    }

    /// Whether the last step ran in parallel windows.
    #[cfg(test)]
    pub(crate) fn ran_parallel(&self) -> bool {
        self.parallel_step
    }
}

impl<S, D, M> StepEvaluator<S, D, M> for AdaptiveEvaluator<S, D, M>
where
    S: PlanningSolution,
    D: Director<S>,
    M: Move<S>,
{
    fn step_started<BestCb>(&mut self, solver_scope: &SolverScope<'_, S, D, BestCb>)
    where
        BestCb: ProgressCallback<S>,
    {
        self.parallel_step = self.policy.is_parallel();
        if !self.parallel_step {
            return;
        }
        let started = Instant::now();
        if self.pool.sync(solver_scope) {
            self.policy.record_replica_build(started.elapsed());
        }
    }

    fn evaluate_candidates<BestCb, C, Resources, A, Fo>(
        &mut self,
        cursor: &mut C,
        resources: &mut Resources,
        step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
        last_step_score: S::Score,
        acceptor: &mut A,
        forager: &mut Fo,
        requires_move_signatures: bool,
        state: CandidateLoopState,
    ) -> CandidateLoopState
    where
        BestCb: ProgressCallback<S>,
        C: ResourceMoveCursor<S, M, Resources>,
        A: Acceptor<S>,
        Fo: LocalSearchForager<S, M>,
    {
        let consumed_before = state.evaluated_moves;
        let trial_time_before = step_scope.phase_scope().stats().evaluation_time();
        let state = if self.parallel_step {
            evaluate_candidate_windows(
                cursor,
                resources,
                step_scope,
                last_step_score,
                acceptor,
                forager,
                requires_move_signatures,
                &mut self.pool,
                state,
            )
        } else {
            let state = sequential::evaluate_candidates(
                cursor,
                resources,
                step_scope,
                last_step_score,
                acceptor,
                forager,
                requires_move_signatures,
                state,
            );
            // Keep the window target current for the first parallel step.
            self.pool
                .finish_step(state.evaluated_moves - consumed_before);
            state
        };
        let trial_time = step_scope
            .phase_scope()
            .stats()
            .evaluation_time()
            .saturating_sub(trial_time_before);
        let next_parallel = self
            .policy
            .observe_step(state.evaluated_moves - consumed_before, trial_time);
        if self.parallel_step && !next_parallel {
            self.pool.release();
        }
        state
    }

    fn step_selected(&mut self, selected: &MoveCandidateRef<'_, S, M>, score: S::Score) {
        if self.parallel_step {
            self.pool.advance(selected, score);
        }
    }

    fn step_committed(&mut self, generation: u64) {
        if self.parallel_step && self.policy.is_parallel() {
            self.pool.mark_synced(generation);
        }
    }
}
