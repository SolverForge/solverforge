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

/// A resolved solver phase plan used as candidate-trace provenance.
///
/// `opaque` is explicit rather than silently falling back to a type name.
/// Consumers must reject an incomplete plan when proving work equivalence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateTracePhasePlan {
    pub kind: String,
    /// Structured, sorted effective settings for this resolved node.  Keys
    /// are unique, which makes the canonical framing unambiguous and prevents
    /// a display-only label from hiding material selector/acceptor/forager or
    /// termination differences.
    pub attributes: Vec<CandidateTracePhaseAttribute>,
    pub opaque: bool,
    pub children: Vec<Self>,
}

/// One canonical resolved-plan attribute.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CandidateTracePhaseAttribute {
    pub key: String,
    pub value: String,
}

impl CandidateTracePhasePlan {
    pub fn known<K, V, I>(kind: impl Into<String>, attributes: I, children: Vec<Self>) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut attributes = attributes
            .into_iter()
            .map(|(key, value)| CandidateTracePhaseAttribute {
                key: key.into(),
                value: value.into(),
            })
            .collect::<Vec<_>>();
        attributes.sort();
        assert!(
            attributes.windows(2).all(|pair| pair[0].key != pair[1].key),
            "candidate trace phase-plan attributes must have unique keys"
        );
        Self {
            kind: kind.into(),
            attributes,
            opaque: false,
            children,
        }
    }

    pub fn opaque(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            attributes: Vec::new(),
            opaque: true,
            children: Vec::new(),
        }
    }

    pub fn is_complete(&self) -> bool {
        !self.opaque && self.children.iter().all(Self::is_complete)
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.append_canonical_bytes(&mut out);
        out
    }

    fn append_canonical_bytes(&self, out: &mut Vec<u8>) {
        out.push(0x50);
        append_string(out, &self.kind);
        append_bool(out, self.opaque);
        append_len(out, self.attributes.len());
        for attribute in &self.attributes {
            append_string(out, &attribute.key);
            append_string(out, &attribute.value);
        }
        append_len(out, self.children.len());
        for child in &self.children {
            child.append_canonical_bytes(out);
        }
    }
}

/// The execution policy the solver actually installed for this run.
///
/// Configuration input is useful provenance, but it is not itself the
/// execution policy: the configured entrypoint can inject a fallback time
/// limit, discard an invalid score target, or compose a criterion with a
/// time guard.  This value records that resolved result explicitly.  A
/// generic [`crate::solver::Solver`] whose termination is supplied as an
/// arbitrary Rust value is intentionally opaque unless its caller supplies a
/// canonical policy through the internal configured-run seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateTraceExecutionPolicy {
    pub kind: String,
    /// Sorted, unique material policy attributes.  These identify the
    /// installed termination composition rather than a display-only summary.
    pub attributes: Vec<CandidateTracePhaseAttribute>,
    pub opaque: bool,
}

impl CandidateTraceExecutionPolicy {
    pub fn known<K, V, I>(kind: impl Into<String>, attributes: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut attributes = attributes
            .into_iter()
            .map(|(key, value)| CandidateTracePhaseAttribute {
                key: key.into(),
                value: value.into(),
            })
            .collect::<Vec<_>>();
        attributes.sort();
        assert!(
            attributes.windows(2).all(|pair| pair[0].key != pair[1].key),
            "candidate trace execution-policy attributes must have unique keys"
        );
        Self {
            kind: kind.into(),
            attributes,
            opaque: false,
        }
    }

    pub fn opaque(kind: impl Into<String>) -> Self {
        Self::opaque_with_attributes(kind, std::iter::empty::<(String, String)>())
    }

    /// Records observable policy fields while explicitly declining to claim
    /// complete policy provenance.  This is used by the generic `Solver`
    /// entrypoint: `with_time_limit` is observable and must be retained, but
    /// the caller's arbitrary Rust termination remains uninspectable.
    pub fn opaque_with_attributes<K, V, I>(kind: impl Into<String>, attributes: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut attributes = attributes
            .into_iter()
            .map(|(key, value)| CandidateTracePhaseAttribute {
                key: key.into(),
                value: value.into(),
            })
            .collect::<Vec<_>>();
        attributes.sort();
        assert!(
            attributes.windows(2).all(|pair| pair[0].key != pair[1].key),
            "candidate trace execution-policy attributes must have unique keys"
        );
        Self {
            kind: kind.into(),
            attributes,
            opaque: true,
        }
    }

    pub const fn is_complete(&self) -> bool {
        !self.opaque
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(0x58);
        append_string(&mut out, &self.kind);
        append_bool(&mut out, self.opaque);
        append_len(&mut out, self.attributes.len());
        for attribute in &self.attributes {
            append_string(&mut out, &attribute.key);
            append_string(&mut out, &attribute.value);
        }
        out
    }
}

/// Digest value supplied by an external fixture or integration boundary.
///
/// SolverForge transports this value; it does not derive it by traversing a
/// model, calling user callbacks, or serializing a working solution. That
/// keeps opt-in diagnostics from changing model work or callback delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CandidateTraceExternalDigest {
    pub bytes: [u8; 32],
}

impl CandidateTraceExternalDigest {
    pub const fn sha256(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    fn append_canonical_bytes(&self, out: &mut Vec<u8>) {
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
    pub fn external(producer: impl Into<String>) -> Self {
        let producer = producer.into();
        assert!(
            !producer.is_empty(),
            "candidate trace external provenance producer must not be empty"
        );
        Self { producer }
    }

    pub fn external_producer(&self) -> &str {
        &self.producer
    }

    fn append_canonical_bytes(&self, out: &mut Vec<u8>) {
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
    pub schema_digest: CandidateTraceExternalDigest,
    pub instance_digest: CandidateTraceExternalDigest,
    pub initial_state_digest: CandidateTraceExternalDigest,
    pub core_tree_digest: Option<CandidateTraceExternalDigest>,
    pub build_digest: Option<CandidateTraceExternalDigest>,
    pub attestation: CandidateTraceInputAttestation,
}

impl CandidateTraceInputProvenance {
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

    pub fn with_core_tree_digest(mut self, digest: CandidateTraceExternalDigest) -> Self {
        self.core_tree_digest = Some(digest);
        self
    }

    pub fn with_build_digest(mut self, digest: CandidateTraceExternalDigest) -> Self {
        self.build_digest = Some(digest);
        self
    }

    fn canonical_bytes(&self) -> Vec<u8> {
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
    Absent,
    ExternallyAttested,
}

/// Separate execution/input provenance status for benchmark qualification.
///
/// This intentionally does not collapse into `CandidateTraceTelemetry::is_complete()`.
/// External attestation remains separate from engine execution-plan
/// completeness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateTraceProvenanceStatus {
    pub execution_policy_complete: bool,
    pub resolved_phase_plan_complete: bool,
    pub input_provenance: CandidateTraceInputProvenanceStatus,
    pub qualification: CandidateTraceQualificationStatus,
}

impl CandidateTraceProvenanceStatus {
    pub const fn has_complete_execution_plan(self) -> bool {
        self.execution_policy_complete && self.resolved_phase_plan_complete
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

/// The engine path that consumed a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CandidateTraceSource {
    Construction,
    LocalSearch,
    VariableNeighborhoodDescent,
    KOpt,
    ListRoundRobinConstruction,
    ListCheapestInsertionTrial,
    ListRegretInsertionTrial,
    ListClarkeWrightSavings,
    ListClarkeWrightMerge,
    ListClarkeWrightCompletionInsertion,
    ListKOptReconnection,
    ListRegretOwnerAppend,
}

impl CandidateTraceSource {
    const fn code(self) -> u8 {
        match self {
            Self::Construction => 1,
            Self::LocalSearch => 2,
            Self::VariableNeighborhoodDescent => 3,
            Self::KOpt => 4,
            Self::ListRoundRobinConstruction => 5,
            Self::ListCheapestInsertionTrial => 6,
            Self::ListRegretInsertionTrial => 7,
            Self::ListClarkeWrightSavings => 8,
            Self::ListClarkeWrightMerge => 9,
            Self::ListClarkeWrightCompletionInsertion => 10,
            Self::ListKOptReconnection => 11,
            Self::ListRegretOwnerAppend => 12,
        }
    }
}

/// One state transition for a retained candidate-pull entry.
///
/// The events are recorded in engine order. This lets a bounded trace prove
/// not only which candidates were pulled, but which one was evaluated,
/// rejected, selected, and actually applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CandidateTraceDisposition {
    /// The engine stopped after the pull and before it evaluated this
    /// candidate.
    InterruptedBeforeEvaluation,
    /// The candidate reached evaluation.
    Evaluated,
    /// Evaluation determined that the candidate was not doable.
    NotDoable,
    /// A hard-improvement requirement rejected the evaluated candidate.
    RejectedByHardImprovement,
    /// A score-improvement requirement rejected the evaluated candidate.
    RejectedByScoreImprovement,
    /// The configured acceptor rejected the evaluated candidate.
    AcceptorRejected,
    /// The candidate was accepted/evaluated but lost to a forager choice.
    ForagerIgnored,
    /// The candidate was selected for commit.
    Selected,
    /// The selected candidate was applied to the working solution.
    Applied,
}

impl CandidateTraceDisposition {
    const fn code(self) -> u8 {
        match self {
            Self::InterruptedBeforeEvaluation => 1,
            Self::Evaluated => 2,
            Self::NotDoable => 3,
            Self::RejectedByHardImprovement => 4,
            Self::RejectedByScoreImprovement => 5,
            Self::AcceptorRejected => 6,
            Self::ForagerIgnored => 7,
            Self::Selected => 8,
            Self::Applied => 9,
        }
    }

    const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::InterruptedBeforeEvaluation
                | Self::NotDoable
                | Self::RejectedByHardImprovement
                | Self::RejectedByScoreImprovement
                | Self::AcceptorRejected
                | Self::ForagerIgnored
                | Self::Applied
        )
    }
}

/// Opaque handle for one retained pull. It exists only after the recorder
/// accepted capacity, so trace-off and overflow paths cannot allocate or
/// update per-candidate diagnostic state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CandidateTracePullToken {
    ordinal: u64,
}

/// A descriptor/entity location that a construction candidate was generated
/// for.  It is absent for local-search candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CandidateTraceConstructionTarget {
    pub descriptor_index: usize,
    pub entity_index: usize,
}

/// Canonical, owned identity for one traced candidate.
///
/// This is deliberately structured rather than debug-formatted or pointer
/// based. The grammar supports a leaf operation with its own logical scope
/// and an ordered composite of independently scoped child identities. That
/// means grouped and cartesian moves do not have to flatten cross-descriptor
/// edits into a misleading parent scope. The framing preserves every field
/// and every list boundary, so a trace consumer can compare exact candidate
/// ordering without relying on a process-local hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateTraceIdentity {
    /// One logical operation scoped to a descriptor and, when applicable, a
    /// planning variable.
    Operation(CandidateTraceOperationIdentity),
    /// An ordered composition of logical child operations. The composite has
    /// no implicit descriptor/variable scope; every child carries its own.
    Composite(CandidateTraceCompositeIdentity),
}

/// Canonical identity for an explicit non-cursor operation performed by a
/// specialized list construction/search engine.
///
/// The operation name is framed verbatim (never hashed), and components are
/// ordered raw coordinates chosen by that engine's actual trial loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateTraceOperationIdentity {
    pub descriptor_index: usize,
    /// Present for a move-family identity and absent for a phase-local
    /// specialized operation that has no variable scope.
    pub variable_name: Option<String>,
    /// Explicit family/operation grammar token, never a pointer, debug value,
    /// or tabu-derived hash.
    pub operation: String,
    pub components: Vec<CandidateTraceCoordinate>,
}

/// Canonical identity for a compound/cartesian move.
///
/// `operation` describes composition semantics (for example
/// `compound_scalar` or `cartesian_product`) while `children` preserves the
/// exact generation/execution order. Children may themselves be composites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateTraceCompositeIdentity {
    pub operation: String,
    pub children: Vec<CandidateTraceIdentity>,
}

/// One explicitly framed logical-coordinate component.
///
/// `Absent` is distinct from every integer value, so an unassigned scalar
/// target never needs a magic sentinel. `Bytes` is reserved for a declared
/// pure typed-value codec; it is never populated from `Debug` output.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CandidateTraceCoordinate {
    Unsigned(u64),
    Absent,
    /// A declared logical token such as a variable name or operation-local
    /// label. It is framed as text, never debug-rendered or hashed.
    Text(String),
    /// A declared pure typed-value codec output. This is intentionally
    /// distinct from [`Self::Text`] and must never be populated from debug or
    /// tabu metadata.
    Bytes(Vec<u8>),
}

impl From<u64> for CandidateTraceCoordinate {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}

impl From<usize> for CandidateTraceCoordinate {
    fn from(value: usize) -> Self {
        Self::Unsigned(value as u64)
    }
}

impl From<Option<usize>> for CandidateTraceCoordinate {
    fn from(value: Option<usize>) -> Self {
        value.map_or(Self::Absent, Self::from)
    }
}

impl From<String> for CandidateTraceCoordinate {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for CandidateTraceCoordinate {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl CandidateTraceIdentity {
    pub fn operation<I, T>(
        descriptor_index: usize,
        operation: impl Into<String>,
        components: I,
    ) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<CandidateTraceCoordinate>,
    {
        Self::Operation(CandidateTraceOperationIdentity {
            descriptor_index,
            variable_name: None,
            operation: operation.into(),
            components: components.into_iter().map(Into::into).collect(),
        })
    }

    pub fn logical_move<I, T>(
        descriptor_index: usize,
        variable_name: impl Into<String>,
        family: impl Into<String>,
        coordinates: I,
    ) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<CandidateTraceCoordinate>,
    {
        Self::Operation(CandidateTraceOperationIdentity {
            descriptor_index,
            variable_name: Some(variable_name.into()),
            operation: family.into(),
            components: coordinates.into_iter().map(Into::into).collect(),
        })
    }

    /// Builds an ordered composite identity without inventing a shared scope
    /// for child operations that may target different descriptors/variables.
    pub fn composite(
        operation: impl Into<String>,
        children: impl IntoIterator<Item = CandidateTraceIdentity>,
    ) -> Self {
        Self::Composite(CandidateTraceCompositeIdentity {
            operation: operation.into(),
            children: children.into_iter().collect(),
        })
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.append_canonical_bytes(&mut out);
        out
    }

    fn append_canonical_bytes(&self, out: &mut Vec<u8>) {
        match self {
            Self::Operation(identity) => {
                out.push(0x4f);
                append_usize(out, identity.descriptor_index);
                match &identity.variable_name {
                    Some(variable_name) => {
                        append_bool(out, true);
                        append_string(out, variable_name);
                    }
                    None => append_bool(out, false),
                }
                append_string(out, &identity.operation);
                append_coordinate_list(out, &identity.components);
            }
            Self::Composite(identity) => {
                out.push(0x43);
                append_string(out, &identity.operation);
                append_len(out, identity.children.len());
                for child in &identity.children {
                    child.append_canonical_bytes(out);
                }
            }
        }
    }
}

/// One ordered candidate pull retained by an enabled trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatePullTelemetry {
    /// Monotonic zero-based ordinal across all instrumented engine pulls.
    pub ordinal: u64,
    pub source: CandidateTraceSource,
    pub phase_index: usize,
    pub phase_type: String,
    pub step_index: u64,
    pub selector_index: Option<usize>,
    /// Source-local cursor/trial coordinate reported by the engine.
    ///
    /// This is useful when inspecting one implementation, but it is not a
    /// cross-representation comparison key: specialized construction loops
    /// can reuse local coordinates across owners or passes. Consumers compare
    /// the globally monotonic [`Self::ordinal`] together with `identity`.
    pub candidate_index: usize,
    pub construction_target: Option<CandidateTraceConstructionTarget>,
    /// `None` explicitly means this move family did not provide a canonical
    /// logical-coordinate identity. Such a trace is not qualifying evidence.
    pub identity: Option<CandidateTraceIdentity>,
    /// Ordered engine decisions for this captured pull. A terminal detail
    /// trace must finish every retained pull with a terminal disposition.
    pub dispositions: Vec<CandidateTraceDisposition>,
}

impl CandidatePullTelemetry {
    fn append_canonical_bytes(&self, out: &mut Vec<u8>) {
        out.push(0x45);
        append_u64(out, self.ordinal);
        out.push(self.source.code());
        append_usize(out, self.phase_index);
        append_string(out, &self.phase_type);
        append_u64(out, self.step_index);
        append_option_usize(out, self.selector_index);
        append_usize(out, self.candidate_index);
        match self.construction_target {
            Some(target) => {
                append_bool(out, true);
                append_usize(out, target.descriptor_index);
                append_usize(out, target.entity_index);
            }
            None => append_bool(out, false),
        }
        match &self.identity {
            Some(identity) => {
                append_bool(out, true);
                identity.append_canonical_bytes(out);
            }
            None => append_bool(out, false),
        }
        append_len(out, self.dispositions.len());
        for disposition in &self.dispositions {
            out.push(disposition.code());
        }
    }

    fn has_terminal_disposition(&self) -> bool {
        self.dispositions
            .last()
            .is_some_and(|disposition| disposition.is_terminal())
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
