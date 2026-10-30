use super::*;

/// Digest value supplied by an external fixture or integration boundary.
///
/// SolverForge transports this value; it does not derive it by traversing a
/// model, calling user callbacks, or serializing a working solution. That
/// keeps opt-in diagnostics from changing model work or callback delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CandidateTraceExternalDigest {
    /// The 32 SHA-256 digest bytes.
    pub bytes: [u8; 32],
}

impl CandidateTraceExternalDigest {
    /// Wraps a SHA-256 digest computed by the caller.
    pub const fn sha256(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    pub(super) fn append_canonical_bytes(&self, out: &mut Vec<u8>) {
        // The fixed 32-byte representation is SHA-256 by contract. Keep the
        // algorithm tag framed so a future explicit algorithm does not
        // silently compare as equivalent.
        out.push(1);
        out.extend_from_slice(&self.bytes);
    }
}

/// Origin of supplied run-input provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateTraceInputAttestation {
    producer: String,
}

impl CandidateTraceInputAttestation {
    /// Records provenance attested by the external `producer`.
    ///
    /// # Panics
    ///
    /// Panics if `producer` is empty.
    pub fn external(producer: impl Into<String>) -> Self {
        let producer = producer.into();
        assert!(
            !producer.is_empty(),
            "candidate trace external provenance producer must not be empty"
        );
        Self { producer }
    }

    /// Name of the external producer that attested the inputs.
    pub fn external_producer(&self) -> &str {
        &self.producer
    }

    pub(super) fn append_canonical_bytes(&self, out: &mut Vec<u8>) {
        out.push(1);
        append_string(out, &self.producer);
    }
}

/// Immutable caller-supplied context for a diagnostic run.
///
/// The three required digests identify the compiled schema/model, source
/// instance, and imported initial working state. Optional core-tree and build
/// digests let a harness pin the executable source as well. This is an
/// attestation hook, not an assertion by SolverForge that two foreign models
/// have equivalent semantics; consumers must inspect [`Self::attestation`]
/// and independently verify external inputs before qualifying a comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateTraceInputProvenance {
    /// Digest of the compiled schema or model.
    pub schema_digest: CandidateTraceExternalDigest,
    /// Digest of the source problem instance.
    pub instance_digest: CandidateTraceExternalDigest,
    /// Digest of the imported initial working state.
    pub initial_state_digest: CandidateTraceExternalDigest,
    /// Optional digest of the core source tree.
    pub core_tree_digest: Option<CandidateTraceExternalDigest>,
    /// Optional digest of the loaded build.
    pub build_digest: Option<CandidateTraceExternalDigest>,
    /// Who attested these digests.
    pub attestation: CandidateTraceInputAttestation,
}

impl CandidateTraceInputProvenance {
    /// Builds provenance from the three required digests attested by
    /// `producer`, without core-tree or build digests.
    ///
    /// # Panics
    ///
    /// Panics if `producer` is empty.
    pub fn externally_attested(
        schema_digest: CandidateTraceExternalDigest,
        instance_digest: CandidateTraceExternalDigest,
        initial_state_digest: CandidateTraceExternalDigest,
        producer: impl Into<String>,
    ) -> Self {
        Self {
            schema_digest,
            instance_digest,
            initial_state_digest,
            core_tree_digest: None,
            build_digest: None,
            attestation: CandidateTraceInputAttestation::external(producer),
        }
    }

    /// Adds the core source-tree digest.
    pub fn with_core_tree_digest(mut self, digest: CandidateTraceExternalDigest) -> Self {
        self.core_tree_digest = Some(digest);
        self
    }

    /// Adds the loaded-build digest.
    pub fn with_build_digest(mut self, digest: CandidateTraceExternalDigest) -> Self {
        self.build_digest = Some(digest);
        self
    }

    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(0x49);
        self.schema_digest.append_canonical_bytes(&mut out);
        self.instance_digest.append_canonical_bytes(&mut out);
        self.initial_state_digest.append_canonical_bytes(&mut out);
        append_optional_external_digest(&mut out, self.core_tree_digest);
        append_optional_external_digest(&mut out, self.build_digest);
        self.attestation.append_canonical_bytes(&mut out);
        out
    }
}

/// Trust state of optional caller-supplied run-input provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateTraceInputProvenanceStatus {
    /// The run supplied no input provenance.
    Absent,
    /// The run supplied provenance attested by an external producer.
    ExternallyAttested,
}

/// Separate execution/input provenance status for benchmark qualification.
///
/// This intentionally does not collapse into `CandidateTraceTelemetry::is_complete()`.
/// External attestation remains separate from engine execution-plan
/// completeness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateTraceProvenanceStatus {
    /// Whether the execution policy is fully known.
    pub execution_policy_complete: bool,
    /// Whether every resolved phase-plan node is known.
    pub resolved_phase_plan_complete: bool,
    /// Whether input provenance was supplied.
    pub input_provenance: CandidateTraceInputProvenanceStatus,
    /// Whether the run requested qualified provenance.
    pub qualification: CandidateTraceQualificationStatus,
}

impl CandidateTraceProvenanceStatus {
    /// Whether both the execution policy and the resolved phase plan are
    /// complete.
    pub const fn has_complete_execution_plan(self) -> bool {
        self.execution_policy_complete && self.resolved_phase_plan_complete
    }
}
