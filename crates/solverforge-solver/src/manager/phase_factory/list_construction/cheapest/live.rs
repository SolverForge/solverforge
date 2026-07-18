//! Live score-director observer for canonical cheapest insertion.

use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::super::ScoredListConstructionAccess;
use super::kernel::{CheapestInsertionObserver, CheapestInsertionTrial};
use crate::phase::construction::record_construction_candidate;
use crate::scope::{PhaseScope, ProgressCallback, StepControlPolicy, StepScope};
use crate::stats::{
    CandidateTraceConstructionTarget, CandidateTraceDisposition, CandidateTracePullToken,
};

/// The sole live publication adapter for the shared cheapest-insertion loop.
pub(crate) struct PhaseCheapestInsertionObserver<'phase, 'termination, 'solver, S, D, BestCb>
where
    S: PlanningSolution,
    D: Director<S>,
{
    phase_scope: &'phase mut PhaseScope<'termination, 'solver, S, D, BestCb>,
    control_policy: StepControlPolicy,
}

impl<'phase, 'termination, 'solver, S, D, BestCb>
    PhaseCheapestInsertionObserver<'phase, 'termination, 'solver, S, D, BestCb>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    pub(crate) fn new(
        phase_scope: &'phase mut PhaseScope<'termination, 'solver, S, D, BestCb>,
        control_policy: StepControlPolicy,
    ) -> Self {
        Self {
            phase_scope,
            control_policy,
        }
    }
}

impl<'phase, 'termination, 'solver, S, A, D, BestCb> CheapestInsertionObserver<S, A>
    for PhaseCheapestInsertionObserver<'phase, 'termination, 'solver, S, D, BestCb>
where
    S: PlanningSolution,
    S::Score: Copy,
    A: ScoredListConstructionAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    type Trial = CandidateTracePullToken;

    fn solution(&self) -> &S {
        self.phase_scope.score_director().working_solution()
    }

    fn should_interrupt_construction(&mut self) -> bool {
        self.control_policy
            .should_terminate_construction(self.phase_scope.solver_scope_mut())
    }

    fn evaluate_insertion(
        &mut self,
        access: &A,
        element: A::Element,
        trial: CheapestInsertionTrial,
    ) -> (Option<S::Score>, Option<Self::Trial>) {
        let descriptor_index = access.descriptor_index();
        let generation_started = Instant::now();
        let trace = self.phase_scope.record_candidate_operation(
            trial.source,
            None,
            trial.candidate_index,
            Some(CandidateTraceConstructionTarget {
                descriptor_index,
                entity_index: trial.entity_index,
            }),
            descriptor_index,
            "list_insertion_trial",
            [
                trial.element_source_index as u64,
                trial.entity_index as u64,
                trial.insertion_index as u64,
            ],
        );
        let generation_duration = generation_started.elapsed();
        let evaluation_started = Instant::now();
        let score_state = self.phase_scope.score_director().snapshot_score_state();
        self.phase_scope
            .score_director_mut()
            .before_variable_changed(descriptor_index, trial.entity_index);
        access.insert_element(
            self.phase_scope.score_director_mut().working_solution_mut(),
            trial.entity_index,
            trial.insertion_index,
            element,
        );
        self.phase_scope
            .score_director_mut()
            .after_variable_changed(descriptor_index, trial.entity_index);
        let score = self.phase_scope.score_director_mut().calculate_score();
        self.phase_scope
            .score_director_mut()
            .before_variable_changed(descriptor_index, trial.entity_index);
        access.remove_element(
            self.phase_scope.score_director_mut().working_solution_mut(),
            trial.entity_index,
            trial.insertion_index,
        );
        self.phase_scope
            .score_director_mut()
            .after_variable_changed(descriptor_index, trial.entity_index);
        self.phase_scope
            .score_director_mut()
            .restore_score_state(score_state);
        self.phase_scope.record_score_calculation();
        if let Some(trace) = trace {
            self.phase_scope
                .record_candidate_trace_disposition(trace, CandidateTraceDisposition::Evaluated);
        }
        record_construction_candidate(
            self.phase_scope,
            generation_duration,
            evaluation_started.elapsed(),
        );
        (Some(score), trace)
    }

    fn discard_trial(&mut self, trial: Self::Trial) {
        self.phase_scope
            .record_candidate_trace_disposition(trial, CandidateTraceDisposition::ForagerIgnored);
    }

    fn select_trial(&mut self, trial: Self::Trial) {
        self.phase_scope
            .record_candidate_trace_disposition(trial, CandidateTraceDisposition::Selected);
    }

    fn commit_insertion(
        &mut self,
        access: &A,
        element: A::Element,
        entity_index: usize,
        insertion_index: usize,
        score: S::Score,
        trace: Option<Self::Trial>,
    ) {
        let descriptor_index = access.descriptor_index();
        let mut step_scope =
            StepScope::new_with_control_policy(self.phase_scope, self.control_policy);
        step_scope.phase_scope_mut().record_move_accepted();
        step_scope.apply_committed_change(|director| {
            director.before_variable_changed(descriptor_index, entity_index);
            access.insert_element(
                director.working_solution_mut(),
                entity_index,
                insertion_index,
                element,
            );
            director.after_variable_changed(descriptor_index, entity_index);
        });
        step_scope.phase_scope_mut().record_move_applied();
        if let Some(trace) = trace {
            step_scope
                .phase_scope_mut()
                .record_candidate_trace_disposition(trace, CandidateTraceDisposition::Applied);
        }
        step_scope.set_step_score(score);
        step_scope.complete();
    }

    fn finish_construction(&mut self) {
        self.phase_scope.update_best_solution();
        self.phase_scope.promote_current_solution_on_score_tie();
    }

    fn finish_without_work(&mut self) {
        self.phase_scope.calculate_score();
        self.phase_scope.update_best_solution();
    }
}
