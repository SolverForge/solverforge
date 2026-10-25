use std::marker::PhantomData;
use std::time::Duration;

use rayon::prelude::*;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::MoveCandidateRef;
use crate::phase::localsearch::evaluation::TrialOutcome;
use crate::phase::localsearch::phase::WindowSlot;
use crate::scope::{ProgressCallback, SolverScope};

/* Largest parallel window. Windows aim at the number of candidates the
previous step consumed: they cover that target in windows of at most this
size, then continue one candidate per evaluator. Candidates past the
decision point are speculative work, so overshooting the step wastes
evaluations.
*/
pub(crate) const MAX_PARALLEL_EVALUATION_WINDOW: usize = 32;

pub(super) type TrialOutputs<Sc> = Vec<(usize, TrialOutcome<Sc>, Duration)>;

/// Trial results of one window, indexed by window slot.
pub(crate) type WindowResults<Sc> = Vec<Option<(TrialOutcome<Sc>, Duration)>>;

/* Evaluators for one local-search phase: the main director plus
`evaluators - 1` persistent replicas, kept at the committed working solution.
The pool never holds more evaluators than one full window has slots, so every
replica can receive a candidate.
*/
pub(crate) struct EvaluatorPool<S, D, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    evaluators: usize,
    replicas: Vec<EvaluatorReplica<S, D, M>>,
    main_outputs: TrialOutputs<S::Score>,
    // Window buffers lent to each step, so steps reuse their capacity.
    window_slots: Vec<WindowSlot>,
    window_results: WindowResults<S::Score>,
    // Candidates the previous step consumed: this step's target.
    last_step_consumed: usize,
    // Candidates dispatched in the current step.
    step_dispatched: usize,
    synced_generation: Option<u64>,
    _phantom: PhantomData<fn() -> (S, M)>,
}

pub(super) struct EvaluatorReplica<S, D, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub(super) director: D,
    pub(super) journal_pool: Vec<M::Undo>,
    pub(super) outputs: TrialOutputs<S::Score>,
}

impl<S, D, M> EvaluatorPool<S, D, M>
where
    S: PlanningSolution,
    D: Director<S>,
    M: Move<S>,
{
    // `requested` counts evaluators, the main director included; it is
    // capped at the window ceiling.
    pub(crate) fn new(requested: usize) -> Self {
        Self {
            evaluators: requested.clamp(1, MAX_PARALLEL_EVALUATION_WINDOW),
            replicas: Vec::new(),
            main_outputs: Vec::new(),
            window_slots: Vec::new(),
            window_results: Vec::new(),
            last_step_consumed: 0,
            step_dispatched: 0,
            synced_generation: None,
            _phantom: PhantomData,
        }
    }

    /// Lends the window buffers to one step; return them with
    /// `restore_window_buffers`.
    pub(crate) fn take_window_buffers(&mut self) -> (Vec<WindowSlot>, WindowResults<S::Score>) {
        (
            std::mem::take(&mut self.window_slots),
            std::mem::take(&mut self.window_results),
        )
    }

    pub(crate) fn restore_window_buffers(
        &mut self,
        mut slots: Vec<WindowSlot>,
        mut results: WindowResults<S::Score>,
    ) {
        slots.clear();
        results.clear();
        self.window_slots = slots;
        self.window_results = results;
    }

    pub(crate) fn is_parallel(&self) -> bool {
        !self.replicas.is_empty()
    }

    pub(crate) fn begin_step(&mut self) {
        self.step_dispatched = 0;
    }

    /// Size of the next window: the rest of the previous step's consumption,
    /// at least one candidate per evaluator and at most
    /// `MAX_PARALLEL_EVALUATION_WINDOW`, capped by the remaining evaluation
    /// budget. Only the parallel loop sizes windows.
    pub(crate) fn window_capacity(&self, remaining: Option<u64>) -> usize {
        let window = self
            .last_step_consumed
            .saturating_sub(self.step_dispatched)
            .clamp(self.evaluators, MAX_PARALLEL_EVALUATION_WINDOW);
        let remaining = remaining.map_or(usize::MAX, |remaining| {
            usize::try_from(remaining).unwrap_or(usize::MAX)
        });
        window.min(remaining).max(1)
    }

    pub(crate) fn record_window(&mut self, len: usize) {
        self.step_dispatched += len;
    }

    pub(crate) fn finish_step(&mut self, consumed: u64) {
        self.last_step_consumed = usize::try_from(consumed).unwrap_or(usize::MAX);
    }

    /* Rebuilds the replicas when the working solution was written since they
    last synced. Each replica clones the working solution once; this is the
    evaluator replica clone boundary.
    */
    pub(crate) fn sync<BestCb>(&mut self, solver_scope: &SolverScope<'_, S, D, BestCb>)
    where
        BestCb: ProgressCallback<S>,
    {
        let generation = solver_scope.working_solution_generation();
        if self.evaluators <= 1 || self.synced_generation == Some(generation) {
            return;
        }
        self.replicas.clear();
        for _ in 1..self.evaluators {
            let director = solver_scope.score_director().fork_replica().expect(
                "parallel evaluation requires a director built with a constraint-set constructor",
            );
            self.replicas.push(EvaluatorReplica {
                director,
                journal_pool: Vec::new(),
                outputs: Vec::new(),
            });
        }
        self.synced_generation = Some(generation);
    }

    /* Applies the selected move to every replica by borrow, ahead of the
    owned commit on the main director, and checks that each replica reaches
    the committed step score.
    */
    pub(crate) fn advance(&mut self, selected: &MoveCandidateRef<'_, S, M>, expected: S::Score) {
        if !self.is_parallel() {
            return;
        }
        self.replicas.par_iter_mut().for_each(|replica| {
            let _undo = selected.do_move(&mut replica.director);
            let score = replica.director.calculate_score();
            assert!(
                score == expected,
                "evaluator replica diverged from the committed step: replica {score}, step {expected}"
            );
        });
    }

    pub(crate) fn mark_synced(&mut self, generation: u64) {
        if self.is_parallel() {
            self.synced_generation = Some(generation);
        }
    }

    pub(super) fn parts_mut(
        &mut self,
    ) -> (
        &mut TrialOutputs<S::Score>,
        &mut [EvaluatorReplica<S, D, M>],
    ) {
        (&mut self.main_outputs, &mut self.replicas)
    }

    #[cfg(test)]
    pub(super) fn replicas(&self) -> impl Iterator<Item = &D> {
        self.replicas.iter().map(|replica| &replica.director)
    }
}
