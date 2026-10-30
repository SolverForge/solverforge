use crate::planning::ScalarEdit;

/// Limits a conflict-repair provider must respect per step.
#[derive(Debug, Clone, Copy)]
pub struct RepairLimits {
    /// Most constraint matches to repair.
    pub max_matches_per_step: usize,
    /// Most repairs per match.
    pub max_repairs_per_match: usize,
    /// Most moves in total.
    pub max_moves_per_step: usize,
}

/// One repair proposed by a provider: a reason and the scalar edits
/// applied together.
#[derive(Debug)]
pub struct RepairCandidate<S> {
    reason: &'static str,
    edits: Vec<ScalarEdit<S>>,
}

impl<S> Clone for RepairCandidate<S> {
    fn clone(&self) -> Self {
        Self {
            reason: self.reason,
            edits: self.edits.clone(),
        }
    }
}

impl<S> PartialEq for RepairCandidate<S> {
    fn eq(&self, other: &Self) -> bool {
        self.reason == other.reason && self.edits == other.edits
    }
}

impl<S> Eq for RepairCandidate<S> {}

impl<S> std::hash::Hash for RepairCandidate<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.reason.hash(state);
        self.edits.hash(state);
    }
}

impl<S> RepairCandidate<S> {
    /// Creates a candidate from a reason label and its edits.
    pub fn new(reason: &'static str, edits: Vec<ScalarEdit<S>>) -> Self {
        Self { reason, edits }
    }

    #[doc(hidden)]
    #[inline]
    pub fn reason(&self) -> &'static str {
        self.reason
    }

    #[doc(hidden)]
    #[inline]
    pub fn edits(&self) -> &[ScalarEdit<S>] {
        &self.edits
    }

    #[doc(hidden)]
    #[inline]
    pub fn into_edits(self) -> Vec<ScalarEdit<S>> {
        self.edits
    }
}

/// Proposes repairs for `solution` within the limits.
pub type RepairProvider<S> = fn(&S, RepairLimits) -> Vec<RepairCandidate<S>>;

/// A repair provider registered for a named constraint.
pub struct ConflictRepair<S> {
    constraint_name: &'static str,
    provider: RepairProvider<S>,
}

impl<S> ConflictRepair<S> {
    /// Registers `provider` for the constraint named `constraint_name`.
    pub const fn new(constraint_name: &'static str, provider: RepairProvider<S>) -> Self {
        Self {
            constraint_name,
            provider,
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn constraint_name(&self) -> &'static str {
        self.constraint_name
    }

    #[doc(hidden)]
    #[inline]
    pub fn provider(&self) -> RepairProvider<S> {
        self.provider
    }
}

impl<S> std::fmt::Debug for ConflictRepair<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConflictRepair")
            .field("constraint_name", &self.constraint_name)
            .finish_non_exhaustive()
    }
}

impl<S> Clone for ConflictRepair<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ConflictRepair<S> {}
