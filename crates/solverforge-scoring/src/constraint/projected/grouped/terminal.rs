use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::DetailedConstraintMatch;
use crate::api::constraint_set::{ConstraintSet, IncrementalConstraint};
use crate::constraint::grouped::GroupedTerminalScorer;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::ConstraintWeight;
use crate::stream::ProjectedSource;

use super::shared_set::SharedProjectedGroupedConstraintSet;
use super::state::ProjectedGroupedNodeState;

type Inner<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc> = SharedProjectedGroupedConstraintSet<
    S,
    Out,
    K,
    Src,
    F,
    KF,
    C,
    V,
    R,
    Acc,
    GroupedTerminalScorer<K, R, W, Sc>,
    Sc,
>;

pub struct ProjectedGroupedConstraint<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
where
    Src: ProjectedSource<S, Out>,
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    is_hard: bool,
    inner: Inner<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>,
    _phantom: PhantomData<fn() -> (V, R, Acc)>,
}

impl<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
    ProjectedGroupedConstraint<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
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
        source: Src,
        filter: F,
        key_fn: KF,
        collector: C,
        weight_fn: W,
        is_hard: bool,
    ) -> Self {
        let state = ProjectedGroupedNodeState::new(source, filter, key_fn, collector);
        let scorer =
            GroupedTerminalScorer::new(constraint_ref.clone(), impact_type, weight_fn, is_hard);
        Self {
            constraint_ref,
            is_hard,
            inner: SharedProjectedGroupedConstraintSet::new(state, scorer),
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W2>(
        self,
        weight: W2,
    ) -> super::shared_set::ProjectedGroupedConstraintSetBuilder<
        S,
        Out,
        K,
        Src,
        F,
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
    ) -> super::shared_set::ProjectedGroupedConstraintSetBuilder<
        S,
        Out,
        K,
        Src,
        F,
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

impl<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc> IncrementalConstraint<S, Sc>
    for ProjectedGroupedConstraint<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate(&self, solution: &S) -> Sc {
        self.inner.evaluate_all(solution)
    }

    fn match_count(&self, solution: &S) -> usize {
        self.inner
            .evaluate_each(solution)
            .first()
            .map_or(0, |result| result.match_count)
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.inner.initialize_all(solution)
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.inner
            .on_insert_all(solution, entity_index, descriptor_index)
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.inner
            .on_retract_all(solution, entity_index, descriptor_index)
    }

    fn reset(&mut self) {
        self.inner.reset_all();
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
