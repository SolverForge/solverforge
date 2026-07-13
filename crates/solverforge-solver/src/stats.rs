mod candidate_trace;
mod candidate_trace_qualified;
mod phase;
mod solver;
mod telemetry;

pub use candidate_trace::{
    CandidatePullTelemetry, CandidateTraceCompositeIdentity, CandidateTraceConstructionTarget,
    CandidateTraceCoordinate, CandidateTraceDigest, CandidateTraceDisposition,
    CandidateTraceExecutionPolicy, CandidateTraceExternalDigest, CandidateTraceHeader,
    CandidateTraceIdentity, CandidateTraceInputAttestation, CandidateTraceInputProvenance,
    CandidateTraceInputProvenanceStatus, CandidateTraceOperationIdentity,
    CandidateTracePhaseAttribute, CandidateTracePhasePlan, CandidateTraceProvenanceStatus,
    CandidateTraceSource, CandidateTraceTelemetry, CANDIDATE_TRACE_FORMAT_VERSION,
};
pub(crate) use candidate_trace::{CandidateTracePullToken, CandidateTraceRecordDecision};
pub use candidate_trace_qualified::{
    CandidateTraceQualificationError, CandidateTraceQualificationStatus,
    QualifiedCandidateTraceRunProvenance,
};
pub use phase::PhaseStats;
pub use solver::SolverStats;
pub(crate) use telemetry::{format_duration, whole_units_per_second};
pub use telemetry::{
    AppliedMoveTelemetry, MoveTelemetry, PhaseTelemetry, SelectorTelemetry, SolverTelemetry,
    Throughput,
};

#[cfg(test)]
mod tests;
