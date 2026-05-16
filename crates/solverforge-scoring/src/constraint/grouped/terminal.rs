use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::DetailedConstraintMatch;
use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::ConstraintWeight;

use super::scorer::GroupedTerminalScorer;
use super::shared_set::SharedGroupedConstraintSet;
use super::state::GroupedNodeState;

type Inner<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc> = SharedGroupedConstraintSet<
    S,
    A,
    K,
    E,
    Fi,
    KF,
    C,
    V,
    R,
    Acc,
    GroupedTerminalScorer<K, R, W, Sc>,
    Sc,
>;

pub struct GroupedUniConstraint<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    is_hard: bool,
    inner: Inner<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>,
    _phantom: PhantomData<fn() -> (S, A, V, R, Acc)>,
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
    GroupedUniConstraint<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    Fi: UniFilter<S, A>,
    KF: Fn(&A) -> K + Send + Sync,
    C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor: E,
        filter: Fi,
        key_fn: KF,
        collector: C,
        weight_fn: W,
        is_hard: bool,
    ) -> Self {
        let node_name = constraint_ref.name.clone();
        let state = GroupedNodeState::new(extractor, filter, key_fn, collector);
        let scorer =
            GroupedTerminalScorer::new(constraint_ref.clone(), impact_type, weight_fn, is_hard);
        Self {
            constraint_ref,
            is_hard,
            inner: SharedGroupedConstraintSet::new(node_name, state, scorer),
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W2>(
        self,
        weight: W2,
    ) -> super::shared_set::GroupedConstraintSetBuilder<
        S,
        A,
        K,
        E,
        Fi,
        KF,
        C,
        V,
        R,
        Acc,
        GroupedTerminalScorer<K, R, W, Sc>,
        impl Fn(&K, &R) -> Sc + Send + Sync,
        Sc,
    >
    where
        W2: for<'w> ConstraintWeight<(&'w K, &'w R), Sc> + Send + Sync,
    {
        self.inner.penalize(weight)
    }

    pub fn reward<W2>(
        self,
        weight: W2,
    ) -> super::shared_set::GroupedConstraintSetBuilder<
        S,
        A,
        K,
        E,
        Fi,
        KF,
        C,
        V,
        R,
        Acc,
        GroupedTerminalScorer<K, R, W, Sc>,
        impl Fn(&K, &R) -> Sc + Send + Sync,
        Sc,
    >
    where
        W2: for<'w> ConstraintWeight<(&'w K, &'w R), Sc> + Send + Sync,
    {
        self.inner.reward(weight)
    }
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc> IncrementalConstraint<S, Sc>
    for GroupedUniConstraint<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    Fi: UniFilter<S, A>,
    KF: Fn(&A) -> K + Send + Sync,
    C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate(&self, solution: &S) -> Sc {
        crate::api::constraint_set::ConstraintSet::evaluate_all(&self.inner, solution)
    }

    fn match_count(&self, solution: &S) -> usize {
        crate::api::constraint_set::ConstraintSet::evaluate_each(&self.inner, solution)
            .first()
            .map_or(0, |result| result.match_count)
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        crate::api::constraint_set::ConstraintSet::initialize_all(&mut self.inner, solution)
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        crate::api::constraint_set::ConstraintSet::on_insert_all(
            &mut self.inner,
            solution,
            entity_index,
            descriptor_index,
        )
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        crate::api::constraint_set::ConstraintSet::on_retract_all(
            &mut self.inner,
            solution,
            entity_index,
            descriptor_index,
        )
    }

    fn reset(&mut self) {
        crate::api::constraint_set::ConstraintSet::reset_all(&mut self.inner);
    }

    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }

    fn get_matches<'a>(&'a self, _solution: &S) -> Vec<DetailedConstraintMatch<'a, Sc>> {
        Vec::new()
    }

    fn weight(&self) -> Sc {
        Sc::zero()
    }
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc> std::fmt::Debug
    for GroupedUniConstraint<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedUniConstraint").finish()
    }
}
