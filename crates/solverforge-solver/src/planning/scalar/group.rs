use super::{
    ScalarAssignmentDeclaration, ScalarAssignmentRule, ScalarCandidateProvider, ScalarTarget,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScalarGroupLimits {
    pub value_candidate_limit: Option<usize>,
    pub group_candidate_limit: Option<usize>,
    pub max_moves_per_step: Option<usize>,
    pub max_augmenting_depth: Option<usize>,
    pub max_rematch_size: Option<usize>,
}

impl ScalarGroupLimits {
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

    pub fn assignment(group_name: &'static str, target: ScalarTarget<S>) -> Self {
        Self {
            group_name,
            targets: vec![target],
            kind: ScalarGroupKind::Assignment(ScalarAssignmentDeclaration::default()),
            limits: ScalarGroupLimits::new(),
        }
    }

    pub fn with_required_entity(mut self, required_entity: fn(&S, usize) -> bool) -> Self {
        self.assignment_mut().required_entity = Some(required_entity);
        self
    }

    pub fn with_capacity_key(
        mut self,
        capacity_key: fn(&S, usize, usize) -> Option<usize>,
    ) -> Self {
        self.assignment_mut().capacity_key = Some(capacity_key);
        self
    }

    pub fn with_assignment_rule(mut self, assignment_rule: ScalarAssignmentRule<S>) -> Self {
        self.assignment_mut().assignment_rule = Some(assignment_rule);
        self
    }

    pub fn with_position_key(mut self, position_key: fn(&S, usize) -> i64) -> Self {
        self.assignment_mut().position_key = Some(position_key);
        self
    }

    pub fn with_sequence_key(
        mut self,
        sequence_key: fn(&S, usize, usize) -> Option<usize>,
    ) -> Self {
        self.assignment_mut().sequence_key = Some(sequence_key);
        self
    }

    pub fn with_entity_order(mut self, entity_order: fn(&S, usize) -> i64) -> Self {
        self.assignment_mut().entity_order = Some(entity_order);
        self
    }

    pub fn with_value_order(mut self, value_order: fn(&S, usize, usize) -> i64) -> Self {
        self.assignment_mut().value_order = Some(value_order);
        self
    }

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
