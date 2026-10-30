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

    /// Finalizes the exact phase plan for an enabled candidate trace.
    ///
    /// A trace-free solve intentionally has no recorder to update. The
    /// one-way invariant itself is enforced by `CandidateTraceTelemetry` when
    /// a trace exists.
    pub(crate) fn finalize_candidate_trace_resolved_phase_plan(
        &mut self,
        resolved_phase_plan: CandidateTracePhasePlan,
    ) {
        if let Some(trace) = self.candidate_trace.as_mut() {
            trace.finalize_resolved_phase_plan(resolved_phase_plan);
        }
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
