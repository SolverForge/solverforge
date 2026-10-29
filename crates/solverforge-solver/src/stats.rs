pub(crate) mod candidate_trace;
pub(crate) mod candidate_trace_qualified;
pub(crate) mod phase;
pub(crate) mod solver;
pub(crate) mod telemetry;

pub(crate) use candidate_trace::{
    CandidatePullTelemetry, CandidateTraceConstructionTarget, CandidateTraceCoordinate,
    CandidateTraceDisposition, CandidateTraceExecutionPolicy, CandidateTraceExternalDigest,
    CandidateTraceHeader, CandidateTraceIdentity, CandidateTraceInputAttestation,
    CandidateTraceInputProvenance, CandidateTracePhasePlan, CandidateTraceSource,
    CandidateTraceTelemetry,
};
pub(crate) use candidate_trace::{CandidateTracePullToken, CandidateTraceRecordDecision};
pub(crate) use candidate_trace_qualified::{
    CandidateTraceQualificationError, CandidateTraceQualificationStatus,
    QualifiedCandidateTraceRunProvenance,
};
pub(crate) use phase::PhaseStats;
pub(crate) use solver::SolverStats;
pub(crate) use telemetry::{format_duration, whole_units_per_second};
pub(crate) use telemetry::{
    AppliedMoveTelemetry, MoveTelemetry, PhaseTelemetry, SelectorTelemetry, SolverTelemetry,
    Throughput,
};

#[cfg(test)]
mod tests;
