use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::DetailedConstraintMatch;
use crate::api::constraint_set::{ConstraintSet, IncrementalConstraint};
use crate::constraint::grouped::GroupedTerminalScorer;
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::ProjectedSource;

use super::shared_set::SharedProjectedComplementedGroupedConstraintSet;
use super::state::ProjectedComplementedGroupedNodeState;

type Inner<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc> =
    SharedProjectedComplementedGroupedConstraintSet<
        S,
        Out,
        B,
        K,
        Src,
        EB,
        F,
        KA,
        KB,
        C,
        V,
        R,
        Acc,
        D,
        GroupedTerminalScorer<K, R, W, Sc>,
        Sc,
    >;

pub struct ProjectedComplementedGroupedConstraint<
    S,
    Out,
    B,
    K,
    Src,
    EB,
    F,
    KA,
    KB,
    C,
    V,
    R,
    Acc,
    D,
    W,
    Sc,
> where
    Src: ProjectedSource<S, Out>,
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    is_hard: bool,
    inner: Inner<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc>,
    _phantom: PhantomData<fn() -> (Out, B, V, R, Acc)>,
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc>
    ProjectedComplementedGroupedConstraint<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    EB: CollectionExtract<S, Item = B>,
    F: UniFilter<S, Out>,
    KA: Fn(&Out) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    D: Fn(&B) -> R + Send + Sync,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        source: Src,
        extractor_b: EB,
        filter: F,
        key_a: KA,
        key_b: KB,
        collector: C,
        default_fn: D,
        weight_fn: W,
        is_hard: bool,
    ) -> Self {
        let state = ProjectedComplementedGroupedNodeState::new(
            source,
            extractor_b,
            filter,
            key_a,
            key_b,
            collector,
            default_fn,
        );
        let scorer =
            GroupedTerminalScorer::new(constraint_ref.clone(), impact_type, weight_fn, is_hard);
        Self {
            constraint_ref,
            is_hard,
            inner: SharedProjectedComplementedGroupedConstraintSet::new(state, scorer),
            _phantom: PhantomData,
        }
    }
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc> IncrementalConstraint<S, Sc>
    for ProjectedComplementedGroupedConstraint<
        S,
        Out,
        B,
        K,
        Src,
        EB,
        F,
        KA,
        KB,
        C,
        V,
        R,
        Acc,
        D,
        W,
        Sc,
    >
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    EB: CollectionExtract<S, Item = B>,
    F: UniFilter<S, Out>,
    KA: Fn(&Out) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    D: Fn(&B) -> R + Send + Sync,
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
