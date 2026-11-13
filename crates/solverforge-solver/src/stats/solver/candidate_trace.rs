//! Candidate-trace recording of solve-wide statistics.

use super::super::{
    CandidatePullTelemetry, CandidateTraceDisposition, CandidateTracePhasePlan,
    CandidateTracePullToken, CandidateTraceRecordDecision, CandidateTraceTelemetry,
};
use super::SolverStats;

impl SolverStats {
    pub(crate) fn enable_candidate_trace(&mut self, candidate_trace: CandidateTraceTelemetry) {
        assert!(
            self.candidate_trace.is_none(),
            "candidate trace may be enabled only once per solve"
        );
        self.candidate_trace = Some(candidate_trace);
    }

    pub(crate) fn prepare_candidate_trace_pull(&mut self) -> CandidateTraceRecordDecision {
        match self.candidate_trace.as_mut() {
            Some(trace) => trace.prepare_pull(),
            None => CandidateTraceRecordDecision::Disabled,
        }
    }

    pub(crate) fn begin_candidate_trace_phase(&mut self) -> Option<usize> {
        self.candidate_trace
            .as_mut()
            .map(CandidateTraceTelemetry::begin_phase)
    }

    /// Records the exact resolved phase plan once, for the final telemetry
    /// and, when enabled, for the candidate trace.
    ///
    /// The one-way invariant is enforced here for every solve and again by
    /// `CandidateTraceTelemetry` when a trace exists.
    pub(crate) fn finalize_resolved_phase_plan(
        &mut self,
        resolved_phase_plan: CandidateTracePhasePlan,
    ) {
        assert!(
            self.resolved_phase_plan.is_none(),
            "the resolved phase plan may be finalized only once per solve"
        );
        if let Some(trace) = self.candidate_trace.as_mut() {
            trace.finalize_resolved_phase_plan(resolved_phase_plan.clone());
        }
        self.resolved_phase_plan = Some(Box::new(resolved_phase_plan));
    }

    pub(crate) fn record_prepared_candidate_trace_pull(
        &mut self,
        pull: CandidatePullTelemetry,
    ) -> CandidateTracePullToken {
        let trace = self
            .candidate_trace
            .as_mut()
            .expect("a prepared candidate trace pull requires an enabled trace");
        trace.push_prepared(pull)
    }

    pub(crate) fn record_candidate_trace_disposition(
        &mut self,
        token: CandidateTracePullToken,
        disposition: CandidateTraceDisposition,
    ) {
        let trace = self
            .candidate_trace
            .as_mut()
            .expect("a candidate trace disposition requires an enabled trace");
        trace.record_disposition(token, disposition);
    }
}
