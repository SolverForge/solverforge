use std::marker::PhantomData;
use std::time::Duration;

use rayon::prelude::*;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveCandidateRef, MoveCandidateUndo};
use crate::phase::localsearch::evaluation::TrialOutcome;
use crate::scope::{ProgressCallback, SolverScope};

/* Candidates per parallel window. Fixed so the trajectory does not depend on
the number of evaluators.
*/
pub(crate) const PARALLEL_EVALUATION_WINDOW: usize = 32;

pub(super) type TrialOutputs<Sc> = Vec<(usize, TrialOutcome<Sc>, Duration)>;

/* Evaluators for one local-search phase: the main director plus
`requested - 1` persistent replicas, kept at the committed working solution.
*/
pub(crate) struct EvaluatorPool<S, D, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    requested: usize,
    replicas: Vec<EvaluatorReplica<S, D, M>>,
    main_outputs: TrialOutputs<S::Score>,
    synced_generation: Option<u64>,
    _phantom: PhantomData<fn() -> (S, M)>,
}

pub(super) struct EvaluatorReplica<S, D, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub(super) director: D,
    pub(super) journal_pool: Vec<MoveCandidateUndo<M::Undo>>,
    pub(super) outputs: TrialOutputs<S::Score>,
}

impl<S, D, M> EvaluatorPool<S, D, M>
where
    S: PlanningSolution,
    D: Director<S>,
    M: Move<S>,
{
    // `requested` counts evaluators, the main director included.
    pub(crate) fn new(requested: usize) -> Self {
        Self {
            requested: requested.max(1),
            replicas: Vec::new(),
            main_outputs: Vec::new(),
            synced_generation: None,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn is_parallel(&self) -> bool {
        !self.replicas.is_empty()
    }

    pub(crate) fn window_capacity(&self, remaining: Option<u64>) -> usize {
        if !self.is_parallel() {
            return 1;
        }
        let remaining = remaining.map_or(usize::MAX, |remaining| {
            usize::try_from(remaining).unwrap_or(usize::MAX)
        });
        PARALLEL_EVALUATION_WINDOW.min(remaining).max(1)
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
        if self.requested <= 1 || self.synced_generation == Some(generation) {
            return;
        }
        self.replicas.clear();
        for _ in 1..self.requested {
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
