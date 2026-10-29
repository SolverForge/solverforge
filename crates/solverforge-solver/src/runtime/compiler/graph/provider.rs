use super::*;

/// Immutable binding for one source inside the common provider cursor.
///
/// Compilation never opens/pulls a provider.  The cursor receives only these
/// frozen handles, so `limited(0)`, unreached union branches, and Cartesian
/// right branches after an impossible left preview cannot observe callback
/// delivery.  A static Rust function and a host callback therefore share one
/// cursor/store/normalizer while retaining their documented ordering policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProviderBindingPlan {
    pub handle: RuntimeProviderHandle,
    /// Immutable source-schema provenance; never substitute a filtered vector
    /// position when emitting trace/provenance.
    pub declared_schema_index: usize,
    pub allowed_slots: Vec<RuntimeScalarSlotId>,
    pub policy: ProviderBindingPolicy,
    pub candidate_contract: ProviderCandidateContract,
}

/// Source-specific behavior interpreted by the one future provider cursor.
/// These variants are data, not selector implementations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProviderBindingPolicy {
    /// Host callback group: normalize/dedupe, take the callback cap (explicit
    /// zero clamps to one), then rotate the resulting move order.
    CallbackGroup {
        rotation_seed_salt: u64,
        pull_timing: ProviderPullTiming,
    },
    /// Existing Rust candidate group: pull/open behavior and rotate-before-
    /// normalization semantics remain explicit while using the same cursor.
    StaticGroup {
        rotation_seed_salt: u64,
        declared_max_moves_per_step: Option<usize>,
        pull_timing: ProviderPullTiming,
    },
    /// Host repair: rotate the full declaration vector before filtering,
    /// invoke a matching multi-constraint provider at most once, and reset
    /// deduplication for every provider result.
    CallbackRepair {
        rotation_seed_salt: u64,
        pull_timing: ProviderPullTiming,
    },
    /// Existing Rust repair: rotate configured constraints, matching provider
    /// indexes, and returned specs with the historic salts; dedup globally
    /// across its configured repair stream.
    StaticRepair {
        constraint_rotation_seed_salt: u64,
        provider_rotation_seed_salt: u64,
        spec_rotation_seed_salt: u64,
        pull_timing: ProviderPullTiming,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProviderPullTiming {
    /// Generic host callbacks are pulled only once the first reachable cursor
    /// `next_candidate` needs them.
    FirstReachableNext,
    /// Native Rust providers open eagerly; callback providers pull only when
    /// their first candidate is reached.
    OpenCursor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ProviderSchedule {
    Group {
        value_candidate_limit: Option<usize>,
        requested_max_moves_per_step: Option<usize>,
    },
    Repair {
        constraints: Vec<String>,
        max_matches_per_step: usize,
        max_repairs_per_match: usize,
        max_moves_per_step: usize,
        include_soft_matches: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CompiledProviderPlan {
    /// The existing dynamic public contract prefixes the tabu/ownership
    /// signature with this kind. Callback reason is deliberately excluded.
    pub move_kind: ProviderMoveKind,
    pub schedule: ProviderSchedule,
    pub bindings: Vec<ProviderBindingPlan>,
}

/// Stable provider-family discriminator retained in move/tabu ownership.
/// Values match the public wrapper's existing dynamic compound signatures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub(crate) enum ProviderMoveKind {
    Grouped = 1,
    ConflictRepair = 2,
    CompoundConflictRepair = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProviderCandidateContract {
    pub reason_storage: ProviderReasonStorage,
    pub deduplication: ProviderCandidateDeduplication,
    pub tabu_identity: ProviderTabuIdentity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProviderReasonStorage {
    /// Provider labels are interned into the solve-owned reason arena when
    /// raw callback output crosses the normalization boundary. Candidate moves
    /// thereafter carry this compact ID, never an `Arc<str>`.
    PerRunInternedId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProviderCandidateDeduplication {
    /// Same ordered edits with a different reason are distinct callback
    /// candidates, exactly as Python's existing declaration contract.
    PerProviderReasonAndOrderedEdits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProviderTabuIdentity {
    /// Tabu keys contain the provider-kind prefix and ordered edit
    /// targets/values.  User callback labels never affect seeded randomness,
    /// but the same edits emitted by Grouped, ConflictRepair, and
    /// CompoundConflictRepair remain distinct move families.
    ProviderKindAndOrderedEdits,
}

pub(in super::super) const GROUP_PROVIDER_ROTATION_SALT: u64 = 0xC0A1_E5CE_DA7A_0001;
pub(in super::super) const REPAIR_PROVIDER_ROTATION_SALT: u64 = 0xC0AF_11C7_DA7A_0001;
pub(in super::super) const STATIC_GROUP_PROVIDER_ROTATION_SALT: u64 = 0xC0A1_E5CE_AAA0_0001;
pub(in super::super) const STATIC_REPAIR_CONSTRAINT_ROTATION_SALT: u64 = 0xC0AF_11C7_0000_0001;
pub(in super::super) const STATIC_REPAIR_PROVIDER_ROTATION_SALT: u64 = 0xC0AF_11C7_0000_0002;
pub(in super::super) const STATIC_REPAIR_SPEC_ROTATION_SALT: u64 = 0xC0AF_11C7_0000_0003;

pub(in super::super) const PYTHON_PROVIDER_CANDIDATE_CONTRACT: ProviderCandidateContract =
    ProviderCandidateContract {
        reason_storage: ProviderReasonStorage::PerRunInternedId,
        deduplication: ProviderCandidateDeduplication::PerProviderReasonAndOrderedEdits,
        tabu_identity: ProviderTabuIdentity::ProviderKindAndOrderedEdits,
    };
