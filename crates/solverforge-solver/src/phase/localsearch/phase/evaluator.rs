//! The evaluation strategy of one local-search phase, chosen once per phase:
//! the one-evaluator loop, or parallel windows across evaluator replicas.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveCandidateRef, ResourceMoveCursor};
use crate::phase::localsearch::parallel::EvaluatorPool;
use crate::phase::localsearch::{Acceptor, LocalSearchForager};
use crate::scope::{ProgressCallback, SolverScope, StepScope};

use super::candidates::{evaluate_candidate_windows, CandidateLoopState};
use super::sequential;

/// How a phase evaluates each step's candidates. Each implementation is its
/// own monomorphized step loop; the phase selects one before its first step.
pub(crate) trait StepEvaluator<S, D, M>
where
    S: PlanningSolution,
    D: Director<S>,
    M: Move<S>,
{
    /// Brings the evaluators to the committed working solution.
    fn step_started<BestCb>(&mut self, solver_scope: &SolverScope<'_, S, D, BestCb>)
    where
        BestCb: ProgressCallback<S>;

    #[allow(clippy::too_many_arguments)]
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
        Fo: LocalSearchForager<S, M>;

    /// Applies the selected move ahead of its owned commit on the main director.
    fn step_selected(&mut self, selected: &MoveCandidateRef<'_, S, M>, score: S::Score);

    /// Records the working-solution generation after the commit.
    fn step_committed(&mut self, generation: u64);
}

/// One evaluator: the main director runs every trial in candidate order.
pub(crate) struct SequentialEvaluator;

impl<S, D, M> StepEvaluator<S, D, M> for SequentialEvaluator
where
    S: PlanningSolution,
    D: Director<S>,
    M: Move<S>,
{
    #[inline]
    fn step_started<BestCb>(&mut self, _solver_scope: &SolverScope<'_, S, D, BestCb>)
    where
        BestCb: ProgressCallback<S>,
    {
    }

    #[inline]
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
        sequential::evaluate_candidates(
            cursor,
            resources,
            step_scope,
            last_step_score,
            acceptor,
            forager,
            requires_move_signatures,
            state,
        )
    }

    #[inline]
    fn step_selected(&mut self, _selected: &MoveCandidateRef<'_, S, M>, _score: S::Score) {}

    #[inline]
    fn step_committed(&mut self, _generation: u64) {}
}

impl<S, D, M> StepEvaluator<S, D, M> for EvaluatorPool<S, D, M>
where
    S: PlanningSolution,
    D: Director<S>,
    M: Move<S>,
{
    fn step_started<BestCb>(&mut self, solver_scope: &SolverScope<'_, S, D, BestCb>)
    where
        BestCb: ProgressCallback<S>,
    {
        self.sync(solver_scope);
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
        evaluate_candidate_windows(
            cursor,
            resources,
            step_scope,
            last_step_score,
            acceptor,
            forager,
            requires_move_signatures,
            self,
            state,
        )
    }

    fn step_selected(&mut self, selected: &MoveCandidateRef<'_, S, M>, score: S::Score) {
        self.advance(selected, score);
    }

    fn step_committed(&mut self, generation: u64) {
        self.mark_synced(generation);
    }
}
