use std::marker::PhantomData;

#[derive(Debug)]
pub struct ScalarTarget<S> {
    descriptor_index: usize,
    variable_name: &'static str,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Clone for ScalarTarget<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ScalarTarget<S> {}

impl<S> PartialEq for ScalarTarget<S> {
    fn eq(&self, other: &Self) -> bool {
        self.descriptor_index == other.descriptor_index && self.variable_name == other.variable_name
    }
}

impl<S> Eq for ScalarTarget<S> {}

impl<S> std::hash::Hash for ScalarTarget<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.descriptor_index.hash(state);
        self.variable_name.hash(state);
    }
}

impl<S> ScalarTarget<S> {
    #[doc(hidden)]
    pub const fn from_descriptor_index(
        descriptor_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            descriptor_index,
            variable_name,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn set(self, entity_index: usize, to_value: Option<usize>) -> ScalarEdit<S> {
        ScalarEdit {
            descriptor_index: self.descriptor_index,
            entity_index,
            variable_name: self.variable_name,
            to_value,
            _phantom: PhantomData,
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn descriptor_index(self) -> usize {
        self.descriptor_index
    }

    #[doc(hidden)]
    #[inline]
    pub fn variable_name(self) -> &'static str {
        self.variable_name
    }
}

#[derive(Debug)]
pub struct ScalarEdit<S> {
    descriptor_index: usize,
    entity_index: usize,
    variable_name: &'static str,
    to_value: Option<usize>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Clone for ScalarEdit<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ScalarEdit<S> {}

impl<S> PartialEq for ScalarEdit<S> {
    fn eq(&self, other: &Self) -> bool {
        self.descriptor_index == other.descriptor_index
            && self.entity_index == other.entity_index
            && self.variable_name == other.variable_name
            && self.to_value == other.to_value
    }
}

impl<S> Eq for ScalarEdit<S> {}

impl<S> std::hash::Hash for ScalarEdit<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.descriptor_index.hash(state);
        self.entity_index.hash(state);
        self.variable_name.hash(state);
        self.to_value.hash(state);
    }
}

impl<S> ScalarEdit<S> {
    #[doc(hidden)]
    pub const fn from_descriptor_index(
        descriptor_index: usize,
        entity_index: usize,
        variable_name: &'static str,
        to_value: Option<usize>,
    ) -> Self {
        Self {
            descriptor_index,
            entity_index,
            variable_name,
            to_value,
            _phantom: PhantomData,
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    #[doc(hidden)]
    #[inline]
    pub fn entity_index(&self) -> usize {
        self.entity_index
    }

    #[doc(hidden)]
    #[inline]
    pub fn variable_name(&self) -> &'static str {
        self.variable_name
    }

    #[doc(hidden)]
    #[inline]
    pub fn to_value(&self) -> Option<usize> {
        self.to_value
    }
}

#[derive(Debug)]
pub struct ScalarCandidate<S> {
    reason: &'static str,
    edits: Vec<ScalarEdit<S>>,
    construction_slot_key: Option<usize>,
    construction_entity_order_key: Option<i64>,
    construction_value_order_key: Option<i64>,
}

impl<S> Clone for ScalarCandidate<S> {
    fn clone(&self) -> Self {
        Self {
            reason: self.reason,
            edits: self.edits.clone(),
            construction_slot_key: self.construction_slot_key,
            construction_entity_order_key: self.construction_entity_order_key,
            construction_value_order_key: self.construction_value_order_key,
        }
    }
}

impl<S> PartialEq for ScalarCandidate<S> {
    fn eq(&self, other: &Self) -> bool {
        self.reason == other.reason
            && self.edits == other.edits
            && self.construction_slot_key == other.construction_slot_key
            && self.construction_entity_order_key == other.construction_entity_order_key
            && self.construction_value_order_key == other.construction_value_order_key
    }
}

impl<S> Eq for ScalarCandidate<S> {}

impl<S> std::hash::Hash for ScalarCandidate<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.reason.hash(state);
        self.edits.hash(state);
        self.construction_slot_key.hash(state);
        self.construction_entity_order_key.hash(state);
        self.construction_value_order_key.hash(state);
    }
}

impl<S> ScalarCandidate<S> {
    pub fn new(reason: &'static str, edits: Vec<ScalarEdit<S>>) -> Self {
        Self {
            reason,
            edits,
            construction_slot_key: None,
            construction_entity_order_key: None,
            construction_value_order_key: None,
        }
    }

    pub fn with_construction_slot_key(mut self, key: usize) -> Self {
        self.construction_slot_key = Some(key);
        self
    }

    pub fn with_construction_entity_order_key(mut self, key: i64) -> Self {
        self.construction_entity_order_key = Some(key);
        self
    }

    pub fn with_construction_value_order_key(mut self, key: i64) -> Self {
        self.construction_value_order_key = Some(key);
        self
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

    #[doc(hidden)]
    #[inline]
    pub fn construction_slot_key(&self) -> Option<usize> {
        self.construction_slot_key
    }

    #[doc(hidden)]
    #[inline]
    pub fn construction_entity_order_key(&self) -> Option<i64> {
        self.construction_entity_order_key
    }

    #[doc(hidden)]
    #[inline]
    pub fn construction_value_order_key(&self) -> Option<i64> {
        self.construction_value_order_key
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScalarGroupLimits {
    pub value_candidate_limit: Option<usize>,
    pub group_candidate_limit: Option<usize>,
    pub max_moves_per_step: Option<usize>,
}

pub type ScalarCandidateProvider<S> = fn(&S, ScalarGroupLimits) -> Vec<ScalarCandidate<S>>;

#[derive(Clone)]
pub struct ScalarGroup<S> {
    group_name: &'static str,
    targets: Vec<ScalarTarget<S>>,
    candidate_provider: ScalarCandidateProvider<S>,
}

impl<S> ScalarGroup<S> {
    pub fn new(
        group_name: &'static str,
        targets: Vec<ScalarTarget<S>>,
        candidate_provider: ScalarCandidateProvider<S>,
    ) -> Self {
        Self {
            group_name,
            targets,
            candidate_provider,
        }
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
    pub fn candidate_provider(&self) -> ScalarCandidateProvider<S> {
        self.candidate_provider
    }
}

impl<S> std::fmt::Debug for ScalarGroup<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScalarGroup")
            .field("group_name", &self.group_name)
            .field("target_count", &self.targets.len())
            .finish_non_exhaustive()
    }
}
