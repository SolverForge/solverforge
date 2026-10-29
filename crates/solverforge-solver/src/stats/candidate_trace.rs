//! Bounded, core-owned candidate-pull diagnostics.
//!
//! This module intentionally records candidates where the solver engine
//! consumes them, rather than decorating selectors or reconstructing a second
//! cursor.  A trace is therefore a faithful ordered prefix of work that
//! reached construction or search evaluation.

use super::{
    CandidateTraceQualificationError, CandidateTraceQualificationStatus,
    QualifiedCandidateTraceRunProvenance,
};

mod identity;
pub(crate) use identity::CandidateTracePullToken;
pub use identity::{
    CandidateTraceCompositeIdentity, CandidateTraceConstructionTarget, CandidateTraceCoordinate,
    CandidateTraceIdentity, CandidateTraceOperationIdentity,
};

mod provenance;
pub use provenance::{
    CandidateTraceExternalDigest, CandidateTraceInputAttestation, CandidateTraceInputProvenance,
    CandidateTraceInputProvenanceStatus, CandidateTraceProvenanceStatus,
};

mod plan;
pub use plan::{
    CandidateTraceExecutionPolicy, CandidateTracePhaseAttribute, CandidateTracePhasePlan,
};

mod pull;
pub use pull::{CandidatePullTelemetry, CandidateTraceDisposition, CandidateTraceSource};

/// Wire-format version for [`CandidateTraceHeader`] and every framed value in
/// this module.
pub const CANDIDATE_TRACE_FORMAT_VERSION: u32 = 3;

/// Stable, non-cryptographic digest used to quickly reject mismatched trace
/// provenance or prefixes.
///
/// The framed configuration, resolved plan, and retained entries remain the
/// source of truth.  This value is a deterministic convenience checksum, not
/// a collision-resistant proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CandidateTraceDigest {
    pub first: u64,
    pub second: u64,
}

impl CandidateTraceDigest {
    const FIRST_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FIRST_PRIME: u64 = 0x0000_0100_0000_01b3;
    const SECOND_OFFSET: u64 = 0x9e37_79b9_7f4a_7c15;
    const SECOND_MULTIPLIER: u64 = 0xd6e8_feb8_6659_fd93;

    pub const fn empty() -> Self {
        Self {
            first: Self::FIRST_OFFSET,
            second: Self::SECOND_OFFSET,
        }
    }

    pub fn of_bytes(bytes: &[u8]) -> Self {
        let mut digest = Self::empty();
        digest.update(bytes);
        digest
    }

    pub fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.first ^= u64::from(byte);
            self.first = self.first.wrapping_mul(Self::FIRST_PRIME);

            self.second ^= u64::from(byte).wrapping_add(0x9d);
            self.second = self
                .second
                .rotate_left(13)
                .wrapping_mul(Self::SECOND_MULTIPLIER)
                .wrapping_add(0x9e37_79b9);
        }
    }
}

impl Default for CandidateTraceDigest {
    fn default() -> Self {
        Self::empty()
    }
}

/// Immutable provenance emitted once for an opt-in candidate trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateTraceHeader {
    pub format_version: u32,
    /// Canonical TOML from the exact `SolverConfig` supplied to the run.
    ///
    /// This is deliberately named *configured input*, not "effective
    /// configuration": effective execution behavior is represented by
    /// [`Self::execution_policy`] and [`Self::resolved_phase_plan`].
    pub configured_input: String,
    pub configured_input_digest: CandidateTraceDigest,
    /// Actual resolved solver policy, including entrypoint defaults injected
    /// by the configured runtime path.
    pub execution_policy: CandidateTraceExecutionPolicy,
    pub execution_policy_digest: CandidateTraceDigest,
    /// False means the execution policy came from an arbitrary generic Rust
    /// termination and cannot qualify as exact work-equivalence evidence.
    pub execution_policy_complete: bool,
    /// Optional immutable model/instance/initial-state provenance supplied by
    /// the entrypoint.
    pub input_provenance: Option<CandidateTraceInputProvenance>,
    pub input_provenance_digest: Option<CandidateTraceDigest>,
    /// Present only when the caller explicitly requested a qualified
    /// diagnostic run. It repeats the exact immutable attestation rather than
    /// requiring consumers to infer qualification from optional fields.
    pub qualified_run_provenance: Option<QualifiedCandidateTraceRunProvenance>,
    pub resolved_phase_plan: CandidateTracePhasePlan,
    pub resolved_phase_plan_digest: CandidateTraceDigest,
    /// False means at least one phase supplied only opaque provenance and
    /// cannot qualify as an exact work-equivalence comparison.
    pub resolved_phase_plan_complete: bool,
}

impl CandidateTraceHeader {
    pub fn new(
        configured_input: String,
        execution_policy: CandidateTraceExecutionPolicy,
        resolved_phase_plan: CandidateTracePhasePlan,
        input_provenance: Option<CandidateTraceInputProvenance>,
    ) -> Self {
        let configured_input_digest = CandidateTraceDigest::of_bytes(configured_input.as_bytes());
        let execution_policy_digest =
            CandidateTraceDigest::of_bytes(&execution_policy.canonical_bytes());
        let execution_policy_complete = execution_policy.is_complete();
        let input_provenance_digest = input_provenance
            .as_ref()
            .map(|provenance| CandidateTraceDigest::of_bytes(&provenance.canonical_bytes()));
        let resolved_phase_plan_digest =
            CandidateTraceDigest::of_bytes(&resolved_phase_plan.canonical_bytes());
        let resolved_phase_plan_complete = resolved_phase_plan.is_complete();
        Self {
            format_version: CANDIDATE_TRACE_FORMAT_VERSION,
            configured_input,
            configured_input_digest,
            execution_policy,
            execution_policy_digest,
            execution_policy_complete,
            input_provenance,
            input_provenance_digest,
            qualified_run_provenance: None,
            resolved_phase_plan,
            resolved_phase_plan_digest,
            resolved_phase_plan_complete,
        }
    }

    pub fn new_qualified(
        configured_input: String,
        execution_policy: CandidateTraceExecutionPolicy,
        resolved_phase_plan: CandidateTracePhasePlan,
        qualified_run_provenance: QualifiedCandidateTraceRunProvenance,
    ) -> Self {
        let mut header = Self::new(
            configured_input,
            execution_policy,
            resolved_phase_plan,
            Some(qualified_run_provenance.input_provenance().clone()),
        );
        header.qualified_run_provenance = Some(qualified_run_provenance);
        header
    }
}

/// Bounded trace emitted in final/pause snapshots for an enabled run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateTraceTelemetry {
    pub header: CandidateTraceHeader,
    pub max_entries: usize,
    /// All instrumented pulls, including pulls beyond `max_entries`.
    pub total_pulls: u64,
    /// The retained ordered prefix.  This is never larger than `max_entries`.
    pub pulls: Vec<CandidatePullTelemetry>,
    /// True when the retained prefix is not the full pull stream.
    pub truncated: bool,
    /// Stable checksum of the retained canonical prefix only.
    pub prefix_digest: CandidateTraceDigest,
    /// Number of retained pulls without an explicit logical-coordinate
    /// identity. A non-zero value makes the trace non-qualifying.
    pub unencoded_identity_count: u64,
    /// A staged runtime may know only a provisional phase plan while it is
    /// paused.  The executor supplies the complete plan once at terminal
    /// completion; do not infer one from configuration or observed pulls.
    resolved_phase_plan_finalized: bool,
    next_phase_index: usize,
}

impl CandidateTraceTelemetry {
    pub(crate) fn new(header: CandidateTraceHeader, max_entries: usize) -> Self {
        Self {
            header,
            max_entries,
            total_pulls: 0,
            // Do not reserve the configured ceiling up front: a diagnostic
            // cap can be intentionally large while a short run pulls only a
            // handful of candidates.
            pulls: Vec::new(),
            truncated: false,
            prefix_digest: CandidateTraceDigest::empty(),
            unencoded_identity_count: 0,
            resolved_phase_plan_finalized: false,
            next_phase_index: 0,
        }
    }

    /// Installs the caller-supplied terminal resolved plan exactly once.
    ///
    /// This deliberately updates only phase-plan provenance and its derived
    /// fields. Configuration, execution policy, input provenance, and
    /// qualified-run attestation remain untouched. In particular, this method
    /// never attempts to reconstruct a plan from candidate pulls or a
    /// partially executed configuration.
    pub(crate) fn finalize_resolved_phase_plan(
        &mut self,
        resolved_phase_plan: CandidateTracePhasePlan,
    ) {
        assert!(
            !self.resolved_phase_plan_finalized,
            "candidate trace resolved phase plan may be finalized only once"
        );
        let resolved_phase_plan_digest =
            CandidateTraceDigest::of_bytes(&resolved_phase_plan.canonical_bytes());
        let resolved_phase_plan_complete = resolved_phase_plan.is_complete();
        self.header.resolved_phase_plan = resolved_phase_plan;
        self.header.resolved_phase_plan_digest = resolved_phase_plan_digest;
        self.header.resolved_phase_plan_complete = resolved_phase_plan_complete;
        self.resolved_phase_plan_finalized = true;
    }

    pub fn is_complete(&self) -> bool {
        !self.truncated
            && self.total_pulls == self.pulls.len() as u64
            && self.unencoded_identity_count == 0
            && self
                .pulls
                .iter()
                .all(CandidatePullTelemetry::has_terminal_disposition)
    }

    /// Whether this trace has complete *engine execution* provenance.
    ///
    /// This is intentionally separate from [`Self::is_complete`]: a bounded
    /// pull stream can be internally complete while the caller has not yet
    /// supplied a verifiable model/instance/initial-state attestation or a
    /// phase has correctly declared its graph opaque.
    pub fn has_complete_execution_provenance(&self) -> bool {
        self.header.execution_policy_complete && self.header.resolved_phase_plan_complete
    }

    pub fn provenance_status(&self) -> CandidateTraceProvenanceStatus {
        let input_provenance = match self.header.input_provenance.as_ref() {
            None => CandidateTraceInputProvenanceStatus::Absent,
            Some(_) => CandidateTraceInputProvenanceStatus::ExternallyAttested,
        };
        CandidateTraceProvenanceStatus {
            execution_policy_complete: self.header.execution_policy_complete,
            resolved_phase_plan_complete: self.header.resolved_phase_plan_complete,
            input_provenance,
            qualification: self
                .header
                .qualified_run_provenance
                .as_ref()
                .map_or(CandidateTraceQualificationStatus::NotRequested, |_| {
                    CandidateTraceQualificationStatus::Qualified
                }),
        }
    }

    /// Returns the immutable attestation only when the caller explicitly
    /// requested qualified diagnostics. A normal trace with optional digests
    /// remains normal and never gets silently upgraded.
    pub fn require_qualified_run_provenance(
        &self,
    ) -> Result<&QualifiedCandidateTraceRunProvenance, CandidateTraceQualificationError> {
        self.header
            .qualified_run_provenance
            .as_ref()
            .ok_or(CandidateTraceQualificationError::QualificationNotRequested)
    }

    pub(crate) fn prepare_pull(&mut self) -> CandidateTraceRecordDecision {
        let ordinal = self.total_pulls;
        self.total_pulls = self.total_pulls.saturating_add(1);
        if self.pulls.len() < self.max_entries {
            CandidateTraceRecordDecision::Capture { ordinal }
        } else {
            self.truncated = true;
            CandidateTraceRecordDecision::Overflow
        }
    }

    pub(crate) fn begin_phase(&mut self) -> usize {
        let phase_index = self.next_phase_index;
        self.next_phase_index = self.next_phase_index.saturating_add(1);
        phase_index
    }

    pub(crate) fn push_prepared(
        &mut self,
        pull: CandidatePullTelemetry,
    ) -> CandidateTracePullToken {
        debug_assert!(self.pulls.len() < self.max_entries);
        debug_assert_eq!(pull.ordinal, self.total_pulls.saturating_sub(1));
        if pull.identity.is_none() {
            self.unencoded_identity_count = self.unencoded_identity_count.saturating_add(1);
        }
        let token = CandidateTracePullToken {
            ordinal: pull.ordinal,
        };
        self.pulls.push(pull);
        token
    }

    pub(crate) fn record_disposition(
        &mut self,
        token: CandidateTracePullToken,
        disposition: CandidateTraceDisposition,
    ) {
        let Some(pull) = self.pulls.get_mut(token.ordinal as usize) else {
            debug_assert!(false, "candidate trace token must point at a retained pull");
            return;
        };
        debug_assert_eq!(pull.ordinal, token.ordinal);
        pull.dispositions.push(disposition);
    }

    /// Takes a coherent diagnostic copy and refreshes the canonical prefix
    /// checksum after all mutable disposition transitions. Normal progress
    /// snapshots never call this because trace detail is deliberately
    /// detached from control-plane telemetry.
    pub(crate) fn snapshot(&self) -> Self {
        let mut snapshot = self.clone();
        snapshot.prefix_digest = CandidateTraceDigest::empty();
        for pull in &snapshot.pulls {
            let mut bytes = Vec::new();
            pull.append_canonical_bytes(&mut bytes);
            snapshot.prefix_digest.update(&bytes);
        }
        snapshot
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CandidateTraceRecordDecision {
    Disabled,
    Capture { ordinal: u64 },
    Overflow,
}

fn append_bool(out: &mut Vec<u8>, value: bool) {
    out.push(u8::from(value));
}

fn append_len(out: &mut Vec<u8>, value: usize) {
    append_u64(
        out,
        u64::try_from(value).expect("candidate trace lengths must fit in u64"),
    );
}

fn append_usize(out: &mut Vec<u8>, value: usize) {
    append_len(out, value);
}

fn append_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn append_option_usize(out: &mut Vec<u8>, value: Option<usize>) {
    match value {
        Some(value) => {
            append_bool(out, true);
            append_usize(out, value);
        }
        None => append_bool(out, false),
    }
}

fn append_optional_external_digest(out: &mut Vec<u8>, value: Option<CandidateTraceExternalDigest>) {
    match value {
        Some(value) => {
            append_bool(out, true);
            value.append_canonical_bytes(out);
        }
        None => append_bool(out, false),
    }
}

fn append_string(out: &mut Vec<u8>, value: &str) {
    append_len(out, value.len());
    out.extend_from_slice(value.as_bytes());
}

fn append_coordinate_list(out: &mut Vec<u8>, values: &[CandidateTraceCoordinate]) {
    append_len(out, values.len());
    for value in values {
        match value {
            CandidateTraceCoordinate::Unsigned(value) => {
                out.push(1);
                append_u64(out, *value);
            }
            CandidateTraceCoordinate::Absent => out.push(2),
            CandidateTraceCoordinate::Text(value) => {
                out.push(3);
                append_string(out, value);
            }
            CandidateTraceCoordinate::Bytes(value) => {
                out.push(4);
                append_len(out, value.len());
                out.extend_from_slice(value);
            }
        }
    }
}
