use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::super::super::constraint::grouped::GroupedUniConstraint;
use super::super::collection_extract::CollectionExtract;
use super::super::collector::UniCollector;
use super::super::filter::UniFilter;

// Zero-erasure builder for finalizing a grouped constraint.
pub struct GroupedConstraintBuilder<S, A, K, E, Fi, KF, C, W, Sc>
where
    Sc: Score,
{
    pub(super) extractor: E,
    pub(super) filter: Fi,
    pub(super) key_fn: KF,
    pub(super) collector: C,
    pub(super) impact_type: ImpactType,
    pub(super) weight_fn: W,
    pub(super) is_hard: bool,
    pub(super) expected_descriptor: Option<usize>,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> K, fn() -> Sc)>,
}

impl<S, A, K, E, Fi, KF, C, W, Sc> GroupedConstraintBuilder<S, A, K, E, Fi, KF, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    Fi: UniFilter<S, A>,
    KF: Fn(&A) -> K + Send + Sync,
    C: UniCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /* Finalizes the builder into a zero-erasure `GroupedUniConstraint`. */
    pub fn named(self, name: &str) -> GroupedUniConstraint<S, A, K, E, Fi, KF, C, W, Sc> {
        let mut constraint = GroupedUniConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.extractor,
            self.filter,
            self.key_fn,
            self.collector,
            self.weight_fn,
            self.is_hard,
        );
        if let Some(d) = self.expected_descriptor {
            constraint = constraint.with_descriptor(d);
        }
        constraint
    }

    pub fn for_descriptor(mut self, descriptor_index: usize) -> Self {
        self.expected_descriptor = Some(descriptor_index);
        self
    }
}

impl<S, A, K, E, Fi, KF, C, W, Sc: Score> std::fmt::Debug
    for GroupedConstraintBuilder<S, A, K, E, Fi, KF, C, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedConstraintBuilder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
