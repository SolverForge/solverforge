use super::{
    ScalarAssignmentDeclaration, ScalarAssignmentRule, ScalarCandidateProvider, ScalarTarget,
};

/// Limits for a scalar group. A `None` field leaves the engine default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScalarGroupLimits {
    /// Most candidate values per entity; `None` means no limit.
    pub value_candidate_limit: Option<usize>,
    /// Most candidates per group pull; `None` means no limit.
    pub group_candidate_limit: Option<usize>,
    /// Most moves per step; `None` lets the grouped selector derive a limit.
    pub max_moves_per_step: Option<usize>,
    /// Deepest augmenting path assignment moves explore; `None` uses 3.
    pub max_augmenting_depth: Option<usize>,
    /// Most rows one assignment rematch may move; `None` uses 4, and smaller
    /// values than 2 count as 2.
    pub max_rematch_size: Option<usize>,
}

impl ScalarGroupLimits {
    /// Creates limits with every field `None`.
    pub const fn new() -> Self {
        Self {
            value_candidate_limit: None,
            group_candidate_limit: None,
            max_moves_per_step: None,
            max_augmenting_depth: None,
            max_rematch_size: None,
        }
    }
}

impl Default for ScalarGroupLimits {
    fn default() -> Self {
        Self::new()
    }
}

/// A named group of scalar variables that construction and local search
/// change together, either from provider candidates or as an assignment.
pub struct ScalarGroup<S> {
    group_name: &'static str,
    targets: Vec<ScalarTarget<S>>,
    kind: ScalarGroupKind<S>,
    limits: ScalarGroupLimits,
}

pub(crate) enum ScalarGroupKind<S> {
    Candidates {
        candidate_provider: ScalarCandidateProvider<S>,
    },
    Assignment(ScalarAssignmentDeclaration<S>),
}

impl<S> Clone for ScalarGroupKind<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ScalarGroupKind<S> {}

impl<S> Clone for ScalarGroup<S> {
    fn clone(&self) -> Self {
        Self {
            group_name: self.group_name,
            targets: self.targets.clone(),
            kind: self.kind,
            limits: self.limits,
        }
    }
}

impl<S> ScalarGroup<S> {
    /// Creates a candidate-backed group over `targets` fed by
    /// `candidate_provider`.
    pub fn candidates(
        group_name: &'static str,
        targets: Vec<ScalarTarget<S>>,
        candidate_provider: ScalarCandidateProvider<S>,
    ) -> Self {
        Self {
            group_name,
            targets,
            kind: ScalarGroupKind::Candidates { candidate_provider },
            limits: ScalarGroupLimits::new(),
        }
    }

    /// Creates an assignment group over one target, which must allow
    /// unassigned values.
    pub fn assignment(group_name: &'static str, target: ScalarTarget<S>) -> Self {
        Self {
            group_name,
            targets: vec![target],
            kind: ScalarGroupKind::Assignment(ScalarAssignmentDeclaration::default()),
            limits: ScalarGroupLimits::new(),
        }
    }

    /// Sets the predicate for rows that must be assigned before the solution
    /// is complete.
    ///
    /// # Panics
    ///
    /// Panics on a candidate-backed group; so do the other assignment hooks.
    pub fn with_required_entity(mut self, required_entity: fn(&S, usize) -> bool) -> Self {
        self.assignment_mut().required_entity = Some(required_entity);
        self
    }

    /// Sets the capacity key consumed by assigning a value to a row; at most
    /// one assigned row may hold each key, and `None` consumes no capacity.
    pub fn with_capacity_key(
        mut self,
        capacity_key: fn(&S, usize, usize) -> Option<usize>,
    ) -> Self {
        self.assignment_mut().capacity_key = Some(capacity_key);
        self
    }

    /// Sets the rule for adjacent rows in a sequence; requires a sequence key.
    pub fn with_assignment_rule(mut self, assignment_rule: ScalarAssignmentRule<S>) -> Self {
        self.assignment_mut().assignment_rule = Some(assignment_rule);
        self
    }

    /// Sets the position of a row used to order assigned rows.
    pub fn with_position_key(mut self, position_key: fn(&S, usize) -> i64) -> Self {
        self.assignment_mut().position_key = Some(position_key);
        self
    }

    /// Sets the position of a `(row, value)` assignment in the value's
    /// sequence; `None` means unsequenced.
    pub fn with_sequence_key(
        mut self,
        sequence_key: fn(&S, usize, usize) -> Option<usize>,
    ) -> Self {
        self.assignment_mut().sequence_key = Some(sequence_key);
        self
    }

    /// Sets the entity ranking key for construction.
    pub fn with_entity_order(mut self, entity_order: fn(&S, usize) -> i64) -> Self {
        self.assignment_mut().entity_order = Some(entity_order);
        self
    }

    /// Sets the value ranking key for construction.
    pub fn with_value_order(mut self, value_order: fn(&S, usize, usize) -> i64) -> Self {
        self.assignment_mut().value_order = Some(value_order);
        self
    }

    /// Replaces the group limits.
    pub fn with_limits(mut self, limits: ScalarGroupLimits) -> Self {
        self.limits = limits;
        self
    }

    fn assignment_mut(&mut self) -> &mut ScalarAssignmentDeclaration<S> {
        let ScalarGroupKind::Assignment(declaration) = &mut self.kind else {
            panic!(
                "scalar group `{}` is candidate-backed; assignment hooks require ScalarGroup::assignment",
                self.group_name
            );
        };
        declaration
    }

    #[doc(hidden)]
    #[inline]
    pub fn group_name(&self) -> &'static str {
        self.group_name
    }

    #[doc(hidden)]
    #[inline]
    pub fn targets(&self) -> &[ScalarTarget<S>] {
        &self.targets
    }

    #[doc(hidden)]
    #[inline]
    pub(crate) fn kind(&self) -> ScalarGroupKind<S> {
        self.kind
    }

    #[doc(hidden)]
    #[inline]
    pub fn limits(&self) -> ScalarGroupLimits {
        self.limits
    }
}

impl<S> std::fmt::Debug for ScalarGroup<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScalarGroup")
            .field("group_name", &self.group_name)
            .field("target_count", &self.targets.len())
            .field(
                "kind",
                match self.kind {
                    ScalarGroupKind::Assignment(_) => &"assignment",
                    ScalarGroupKind::Candidates { .. } => &"candidates",
                },
            )
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}
