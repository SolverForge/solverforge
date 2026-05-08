use std::marker::PhantomData;

use crate::planning::ScalarTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageGroupLimits {
    /// Caps candidate values considered for each entity in construction and repair.
    pub value_candidate_limit: Option<usize>,
    /// Caps group-level construction candidates. Repair selectors do not read
    /// this limit.
    pub group_candidate_limit: Option<usize>,
    /// Caps moves emitted by coverage repair selectors. Construction does not
    /// read this limit.
    pub max_moves_per_step: Option<usize>,
    /// Caps the recursive augmenting path depth used while relocating blockers.
    pub max_augmenting_depth: Option<usize>,
}

impl CoverageGroupLimits {
    pub const fn new() -> Self {
        Self {
            value_candidate_limit: None,
            group_candidate_limit: None,
            max_moves_per_step: None,
            max_augmenting_depth: None,
        }
    }
}

impl Default for CoverageGroupLimits {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CoverageGroup<S> {
    group_name: &'static str,
    target: ScalarTarget<S>,
    required_slot: Option<fn(&S, usize) -> bool>,
    capacity_key: Option<fn(&S, usize, usize) -> Option<usize>>,
    entity_order: Option<fn(&S, usize) -> i64>,
    value_order: Option<fn(&S, usize, usize) -> i64>,
    limits: CoverageGroupLimits,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Clone for CoverageGroup<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for CoverageGroup<S> {}

impl<S> CoverageGroup<S> {
    pub const fn new(group_name: &'static str, target: ScalarTarget<S>) -> Self {
        Self {
            group_name,
            target,
            required_slot: None,
            capacity_key: None,
            entity_order: None,
            value_order: None,
            limits: CoverageGroupLimits::new(),
            _phantom: PhantomData,
        }
    }

    pub const fn with_required_slot(mut self, required_slot: fn(&S, usize) -> bool) -> Self {
        self.required_slot = Some(required_slot);
        self
    }

    pub const fn with_capacity_key(
        mut self,
        capacity_key: fn(&S, usize, usize) -> Option<usize>,
    ) -> Self {
        self.capacity_key = Some(capacity_key);
        self
    }

    pub const fn with_entity_order(mut self, entity_order: fn(&S, usize) -> i64) -> Self {
        self.entity_order = Some(entity_order);
        self
    }

    pub const fn with_value_order(mut self, value_order: fn(&S, usize, usize) -> i64) -> Self {
        self.value_order = Some(value_order);
        self
    }

    pub const fn with_limits(mut self, limits: CoverageGroupLimits) -> Self {
        self.limits = limits;
        self
    }

    #[doc(hidden)]
    #[inline]
    pub fn group_name(&self) -> &'static str {
        self.group_name
    }

    #[doc(hidden)]
    #[inline]
    pub fn target(&self) -> ScalarTarget<S> {
        self.target
    }

    #[doc(hidden)]
    #[inline]
    pub fn required_slot(&self) -> Option<fn(&S, usize) -> bool> {
        self.required_slot
    }

    #[doc(hidden)]
    #[inline]
    pub fn capacity_key(&self) -> Option<fn(&S, usize, usize) -> Option<usize>> {
        self.capacity_key
    }

    #[doc(hidden)]
    #[inline]
    pub fn entity_order(&self) -> Option<fn(&S, usize) -> i64> {
        self.entity_order
    }

    #[doc(hidden)]
    #[inline]
    pub fn value_order(&self) -> Option<fn(&S, usize, usize) -> i64> {
        self.value_order
    }

    #[doc(hidden)]
    #[inline]
    pub fn limits(&self) -> CoverageGroupLimits {
        self.limits
    }
}

impl<S> std::fmt::Debug for CoverageGroup<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoverageGroup")
            .field("group_name", &self.group_name)
            .field("target_descriptor_index", &self.target.descriptor_index())
            .field("target_variable_name", &self.target.variable_name())
            .field("has_required_slot", &self.required_slot.is_some())
            .field("has_capacity_key", &self.capacity_key.is_some())
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}
