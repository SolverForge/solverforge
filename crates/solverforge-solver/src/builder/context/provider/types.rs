use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::super::RuntimeScalarSlot;
use crate::{ConflictRepair, ScalarCandidateProvider, ScalarGroupLimits};

/// One raw callback edit. It deliberately has names, not descriptor indexes
/// or pre-bound slots: the core resolver binds it after callback return.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RawProviderEdit {
    pub entity_class: Option<Arc<str>>,
    pub variable_name: Arc<str>,
    pub entity_index: usize,
    pub to_value: Option<usize>,
}

/// One raw callback candidate. The reason is owned and participates in
/// per-provider deduplication, but never in tabu identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RawProviderCandidate {
    pub reason: Arc<str>,
    pub edits: Vec<RawProviderEdit>,
}

/// Compact identity for one callback/provider reason within a single solve.
///
/// The raw callback boundary may own arbitrary strings, but candidate moves
/// must not clone or refcount a label. A runtime execution owns one arena and
/// hands its mutable reference to each provider cursor it instantiates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProviderReasonId(u32);

#[derive(Clone, Debug)]
enum ProviderReasonLabel {
    Static(&'static str),
    Host(Arc<str>),
}

impl AsRef<str> for ProviderReasonLabel {
    fn as_ref(&self) -> &str {
        match self {
            Self::Static(label) => label,
            Self::Host(label) => label,
        }
    }
}

impl Borrow<str> for ProviderReasonLabel {
    fn borrow(&self) -> &str {
        self.as_ref()
    }
}

impl PartialEq for ProviderReasonLabel {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl Eq for ProviderReasonLabel {}

impl Hash for ProviderReasonLabel {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

pub(super) enum ProviderCandidateReason {
    Static(&'static str),
    Host(Arc<str>),
}

/// Per-run reason interning. This is deliberately separate from compiled
/// schema/config: host labels are run data, while static Rust labels remain
/// borrowed `&'static str`. One compiled-selector execution owns this arena
/// and lends a mutable reference to provider cursors only while they normalize
/// a pull, keeping shared ownership out of the move hot path.
#[derive(Debug, Default)]
pub struct ProviderReasonArena {
    ids: HashMap<ProviderReasonLabel, ProviderReasonId>,
    labels: Vec<ProviderReasonLabel>,
}

impl ProviderReasonArena {
    pub fn intern(&mut self, label: Arc<str>) -> ProviderReasonId {
        self.intern_label(ProviderReasonLabel::Host(label))
    }

    pub(crate) fn intern_static(&mut self, label: &'static str) -> ProviderReasonId {
        self.intern_label(ProviderReasonLabel::Static(label))
    }

    pub(super) fn intern_candidate(&mut self, reason: ProviderCandidateReason) -> ProviderReasonId {
        match reason {
            ProviderCandidateReason::Static(label) => self.intern_static(label),
            ProviderCandidateReason::Host(label) => self.intern(label),
        }
    }

    fn intern_label(&mut self, label: ProviderReasonLabel) -> ProviderReasonId {
        if let Some(id) = self.ids.get(label.as_ref()) {
            return *id;
        }

        let index = self.labels.len();
        let id = ProviderReasonId(
            u32::try_from(index).expect("a single solve cannot intern more than u32::MAX reasons"),
        );
        self.labels.push(label.clone());
        self.ids.insert(label, id);
        id
    }

    pub fn label(&self, id: ProviderReasonId) -> &str {
        self.labels
            .get(id.0 as usize)
            .map(ProviderReasonLabel::as_ref)
            .expect("runtime compound move refers to a reason outside its run arena")
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.labels.len()
    }
}

/// The immutable callback request supplied by the one shared provider kernel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeProviderLimits {
    Group {
        value_candidate_limit: Option<usize>,
        max_moves_per_step: Option<usize>,
    },
    Repair {
        constraints: Arc<[Arc<str>]>,
        max_matches_per_step: usize,
        max_repairs_per_match: usize,
        max_moves_per_step: usize,
        include_soft_matches: bool,
    },
}

/// Immutable source handle consumed by the one runtime-provider cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RuntimeProviderHandle {
    CallbackGroup(usize),
    StaticGroup(usize),
    CallbackRepair(usize),
    StaticRepair(usize),
}

/// Object-safe lazy pull boundary for host-language callbacks only.
///
/// A Python implementation performs `Python::attach(...).unwrap_or_else(
/// panic_with_py_err)` inside this method. Core never catches/re-wraps that
/// panic, preserving the original Python exception and traceback for direct
/// and retained solves. `open_cursor`, `size`, and `validate_cursor` must not
/// call this method.
pub trait RuntimeHostCompoundProvider<S>: Send + Sync {
    fn pull(&self, solution: &S, limits: RuntimeProviderLimits) -> Vec<RawProviderCandidate>;
}

/// Structured core normalization failure. A host-provided error boundary
/// converts it to its native exception type at pull time; core never formats
/// or otherwise replaces callback failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderResolutionError {
    UnknownSlot {
        entity_class: Option<Arc<str>>,
        variable_name: Arc<str>,
    },
    SlotOutsideSelector {
        entity_class: Arc<str>,
        variable_name: Arc<str>,
    },
    EntityIndexOutOfBounds {
        entity_class: Arc<str>,
        variable_name: Arc<str>,
        entity_index: usize,
    },
    IllegalValue {
        entity_class: Arc<str>,
        variable_name: Arc<str>,
        entity_index: usize,
        to_value: Option<usize>,
    },
}

impl fmt::Display for ProviderResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSlot {
                entity_class,
                variable_name,
            } => write!(
                f,
                "compound provider edit targets unknown scalar variable `{}`{}",
                variable_name,
                entity_class
                    .as_ref()
                    .map(|entity| format!(" on `{entity}`"))
                    .unwrap_or_default()
            ),
            Self::SlotOutsideSelector {
                entity_class,
                variable_name,
            } => write!(
                f,
                "compound provider edit targets `{entity_class}.{variable_name}` outside this compiled provider selector"
            ),
            Self::EntityIndexOutOfBounds {
                entity_class,
                variable_name,
                entity_index,
            } => write!(
                f,
                "compound provider edit entity_index `{entity_index}` is out of bounds for `{entity_class}.{variable_name}`"
            ),
            Self::IllegalValue {
                entity_class,
                variable_name,
                entity_index,
                to_value,
            } => write!(
                f,
                "compound provider edit value {to_value:?} is not legal for `{entity_class}.{variable_name}` row `{entity_index}`"
            ),
        }
    }
}

/// Host exception boundary for core normalization errors. Python bindings
/// implement this by calling their existing `panic_with_py_err(py_err(...))`.
pub trait RuntimeHostProviderErrorBoundary: Send + Sync {
    fn raise(&self, error: ProviderResolutionError) -> !;
}

#[derive(Debug)]
pub(super) struct PanicProviderErrorBoundary;

impl RuntimeHostProviderErrorBoundary for PanicProviderErrorBoundary {
    fn raise(&self, error: ProviderResolutionError) -> ! {
        panic!("{error}")
    }
}

/// One host-language callback group declaration in immutable schema order.
pub struct RuntimeScalarGroupProviderBinding<S> {
    pub declared_index: usize,
    pub group_name: Arc<str>,
    pub callback: Arc<dyn RuntimeHostCompoundProvider<S>>,
}

impl<S> Clone for RuntimeScalarGroupProviderBinding<S> {
    fn clone(&self) -> Self {
        Self {
            declared_index: self.declared_index,
            group_name: Arc::clone(&self.group_name),
            callback: Arc::clone(&self.callback),
        }
    }
}

impl<S> fmt::Debug for RuntimeScalarGroupProviderBinding<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeScalarGroupProviderBinding")
            .field("declared_index", &self.declared_index)
            .field("group_name", &self.group_name)
            .finish_non_exhaustive()
    }
}

/// One host-language conflict-repair declaration in immutable schema order.
/// A multi-constraint declaration retains one index and one pull.
pub struct RuntimeConflictRepairProviderBinding<S> {
    pub declared_index: usize,
    pub declared_constraints: Arc<[Arc<str>]>,
    pub callback: Arc<dyn RuntimeHostCompoundProvider<S>>,
}

impl<S> Clone for RuntimeConflictRepairProviderBinding<S> {
    fn clone(&self) -> Self {
        Self {
            declared_index: self.declared_index,
            declared_constraints: Arc::clone(&self.declared_constraints),
            callback: Arc::clone(&self.callback),
        }
    }
}

impl<S> fmt::Debug for RuntimeConflictRepairProviderBinding<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeConflictRepairProviderBinding")
            .field("declared_index", &self.declared_index)
            .field("declared_constraints", &self.declared_constraints)
            .finish_non_exhaustive()
    }
}

/// One immutable function-pointer binding for a typed Rust candidate group.
pub struct StaticScalarGroupProviderBinding<S> {
    pub declared_index: usize,
    pub group_name: &'static str,
    pub provider: ScalarCandidateProvider<S>,
    pub declared_limits: ScalarGroupLimits,
}

impl<S> Clone for StaticScalarGroupProviderBinding<S> {
    fn clone(&self) -> Self {
        Self {
            declared_index: self.declared_index,
            group_name: self.group_name,
            provider: self.provider,
            declared_limits: self.declared_limits,
        }
    }
}

impl<S> fmt::Debug for StaticScalarGroupProviderBinding<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StaticScalarGroupProviderBinding")
            .field("declared_index", &self.declared_index)
            .field("group_name", &self.group_name)
            .finish_non_exhaustive()
    }
}

/// One immutable function-pointer binding for a typed Rust repair provider.
pub struct StaticConflictRepairProviderBinding<S> {
    pub declared_index: usize,
    pub repair: ConflictRepair<S>,
}

impl<S> Clone for StaticConflictRepairProviderBinding<S> {
    fn clone(&self) -> Self {
        Self {
            declared_index: self.declared_index,
            repair: self.repair,
        }
    }
}

impl<S> fmt::Debug for StaticConflictRepairProviderBinding<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StaticConflictRepairProviderBinding")
            .field("declared_index", &self.declared_index)
            .field("constraint_name", &self.repair.constraint_name())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct ResolvedProviderEdit<S> {
    pub descriptor_index: usize,
    pub variable_index: usize,
    pub slot: RuntimeScalarSlot<S>,
    pub entity_index: usize,
    pub to_value: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct ResolvedProviderCandidate<S> {
    pub reason: ProviderReasonId,
    pub edits: Vec<ResolvedProviderEdit<S>>,
}

/// Deduplication state for one explicit provider result stream.
#[derive(Debug, Default)]
pub struct ProviderNormalizationState {
    pub(super) seen_candidates:
        HashSet<(ProviderReasonId, Vec<(usize, usize, usize, Option<usize>)>)>,
}
