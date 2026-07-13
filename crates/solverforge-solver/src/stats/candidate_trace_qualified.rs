//! Explicit, fail-closed provenance for a qualified diagnostic run.
//!
//! Normal candidate tracing accepts optional caller attestations. A benchmark
//! that claims loaded-binary work equivalence needs more: every immutable input
//! digest, the exact core tree, the loaded build, and a non-empty external
//! producer. This value represents that deliberate stronger request without
//! changing ordinary tracing semantics or deriving any value from callbacks or
//! working state.

use std::fmt;

use super::{
    CandidateTraceExternalDigest, CandidateTraceInputAttestation, CandidateTraceInputProvenance,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateTraceQualificationError {
    QualificationNotRequested,
    EmptyExternalProducer,
    MissingCoreTreeDigest,
    MissingBuildDigest,
}

impl fmt::Display for CandidateTraceQualificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QualificationNotRequested => {
                write!(f, "qualified candidate-trace provenance was not requested")
            }
            Self::EmptyExternalProducer => {
                write!(f, "qualified candidate-trace producer must not be empty")
            }
            Self::MissingCoreTreeDigest => write!(
                f,
                "qualified candidate-trace provenance requires a core-tree digest"
            ),
            Self::MissingBuildDigest => write!(
                f,
                "qualified candidate-trace provenance requires a loaded-build digest"
            ),
        }
    }
}

impl std::error::Error for CandidateTraceQualificationError {}

/// Whether a trace explicitly requested qualified run provenance.
///
/// This is intentionally separate from optional input provenance: ordinary
/// diagnostics may include digests but must not be treated as fail-closed
/// qualification unless the qualified run entrypoint installed this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateTraceQualificationStatus {
    NotRequested,
    Qualified,
}

/// Immutable attestation required before a caller may request a qualified
/// diagnostic run.
///
/// The constructor takes all five source/build digests directly. It does not
/// read environment variables, serialize a solution, or invoke model code;
/// a harness obtains its digests before calling SolverForge and passes this
/// value through the bridge unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedCandidateTraceRunProvenance {
    input: CandidateTraceInputProvenance,
}

impl QualifiedCandidateTraceRunProvenance {
    #[allow(clippy::too_many_arguments)]
    pub fn externally_attested(
        schema_digest: CandidateTraceExternalDigest,
        instance_digest: CandidateTraceExternalDigest,
        initial_state_digest: CandidateTraceExternalDigest,
        core_tree_digest: CandidateTraceExternalDigest,
        build_digest: CandidateTraceExternalDigest,
        producer: impl Into<String>,
    ) -> Result<Self, CandidateTraceQualificationError> {
        let producer = producer.into();
        if producer.trim().is_empty() {
            return Err(CandidateTraceQualificationError::EmptyExternalProducer);
        }
        Ok(Self {
            input: CandidateTraceInputProvenance {
                schema_digest,
                instance_digest,
                initial_state_digest,
                core_tree_digest: Some(core_tree_digest),
                build_digest: Some(build_digest),
                attestation: CandidateTraceInputAttestation::external(producer),
            },
        })
    }

    /// Upgrades an existing externally supplied provenance only when it has
    /// the full immutable attestation a qualified run requires.
    pub fn try_from_input(
        input: CandidateTraceInputProvenance,
    ) -> Result<Self, CandidateTraceQualificationError> {
        let producer = input.attestation.external_producer();
        if producer.trim().is_empty() {
            return Err(CandidateTraceQualificationError::EmptyExternalProducer);
        }
        if input.core_tree_digest.is_none() {
            return Err(CandidateTraceQualificationError::MissingCoreTreeDigest);
        }
        if input.build_digest.is_none() {
            return Err(CandidateTraceQualificationError::MissingBuildDigest);
        }
        Ok(Self { input })
    }

    pub fn input_provenance(&self) -> &CandidateTraceInputProvenance {
        &self.input
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::{
        CandidateTraceExecutionPolicy, CandidateTraceHeader, CandidateTracePhasePlan,
        CandidateTraceQualificationStatus, CandidateTraceTelemetry,
    };

    fn digest(byte: u8) -> CandidateTraceExternalDigest {
        CandidateTraceExternalDigest::sha256([byte; 32])
    }

    fn qualified() -> QualifiedCandidateTraceRunProvenance {
        QualifiedCandidateTraceRunProvenance::externally_attested(
            digest(1),
            digest(2),
            digest(3),
            digest(4),
            digest(5),
            "solverforge-bench",
        )
        .expect("all immutable digests and a producer qualify the run")
    }

    #[test]
    fn qualification_rejects_an_empty_producer_before_execution() {
        assert_eq!(
            QualifiedCandidateTraceRunProvenance::externally_attested(
                digest(1),
                digest(2),
                digest(3),
                digest(4),
                digest(5),
                "  ",
            ),
            Err(CandidateTraceQualificationError::EmptyExternalProducer),
        );
    }

    #[test]
    fn optional_provenance_cannot_silently_upgrade_to_qualified() {
        let input = CandidateTraceInputProvenance::externally_attested(
            digest(1),
            digest(2),
            digest(3),
            "solverforge-bench",
        );
        assert_eq!(
            QualifiedCandidateTraceRunProvenance::try_from_input(input),
            Err(CandidateTraceQualificationError::MissingCoreTreeDigest),
        );
    }

    #[test]
    fn qualified_header_exposes_exact_attestation_and_normal_header_stays_unqualified() {
        let qualified = qualified();
        let header = CandidateTraceHeader::new_qualified(
            "[candidate_trace]\nmax_entries = 1\n".to_string(),
            CandidateTraceExecutionPolicy::known("test", std::iter::empty::<(String, String)>()),
            CandidateTracePhasePlan::known(
                "test",
                std::iter::empty::<(String, String)>(),
                Vec::new(),
            ),
            qualified.clone(),
        );
        let trace = CandidateTraceTelemetry::new(header, 1);
        assert_eq!(trace.require_qualified_run_provenance(), Ok(&qualified));
        assert_eq!(
            trace.provenance_status().qualification,
            CandidateTraceQualificationStatus::Qualified,
        );

        let normal = CandidateTraceTelemetry::new(
            CandidateTraceHeader::new(
                "[candidate_trace]\nmax_entries = 1\n".to_string(),
                CandidateTraceExecutionPolicy::known(
                    "test",
                    std::iter::empty::<(String, String)>(),
                ),
                CandidateTracePhasePlan::known(
                    "test",
                    std::iter::empty::<(String, String)>(),
                    Vec::new(),
                ),
                Some(qualified.input_provenance().clone()),
            ),
            1,
        );
        assert_eq!(
            normal.require_qualified_run_provenance(),
            Err(CandidateTraceQualificationError::QualificationNotRequested),
        );
        assert_eq!(
            normal.provenance_status().qualification,
            CandidateTraceQualificationStatus::NotRequested,
        );
    }

    #[test]
    fn finalizing_a_plan_preserves_qualified_run_attestation() {
        let qualified = qualified();
        let mut trace = CandidateTraceTelemetry::new(
            CandidateTraceHeader::new_qualified(
                "[candidate_trace]\nmax_entries = 1\n".to_string(),
                CandidateTraceExecutionPolicy::known(
                    "test",
                    std::iter::empty::<(String, String)>(),
                ),
                CandidateTracePhasePlan::opaque("test.pending"),
                qualified.clone(),
            ),
            1,
        );

        let terminal_plan = CandidateTracePhasePlan::known(
            "test.terminal",
            std::iter::empty::<(String, String)>(),
            Vec::new(),
        );
        trace.finalize_resolved_phase_plan(terminal_plan.clone());

        assert_eq!(trace.header.resolved_phase_plan, terminal_plan);
        assert!(trace.header.resolved_phase_plan_complete);
        assert_eq!(trace.require_qualified_run_provenance(), Ok(&qualified));
        assert_eq!(
            trace.provenance_status().qualification,
            CandidateTraceQualificationStatus::Qualified,
        );
    }
}
