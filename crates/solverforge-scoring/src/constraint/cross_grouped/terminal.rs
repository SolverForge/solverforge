use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::DetailedConstraintMatch;
use crate::api::constraint_set::{ConstraintSet, IncrementalConstraint};
use crate::constraint::grouped::GroupedTerminalScorer;
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::ConstraintWeight;

use super::shared_set::SharedCrossGroupedConstraintSet;
use super::state::CrossGroupedNodeState;

type Inner<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc> =
    SharedCrossGroupedConstraintSet<
        S,
        A,
        B,
        JK,
        GK,
        EA,
        EB,
        KA,
        KB,
        F,
        GF,
        C,
        V,
        R,
        Acc,
        GroupedTerminalScorer<GK, R, W, Sc>,
        Sc,
    >;

pub struct CrossGroupedConstraint<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    is_hard: bool,
    inner: Inner<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>,
    _phantom: PhantomData<fn() -> (A, B, V, R, Acc)>,
}

impl<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
    CrossGroupedConstraint<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    GK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + Send + Sync,
    EB: CollectionExtract<S, Item = B> + Send + Sync,
    KA: Fn(&A) -> JK + Send + Sync,
    KB: Fn(&B) -> JK + Send + Sync,
    F: Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
    GF: Fn(&A, &B) -> GK + Send + Sync,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    W: Fn(&GK, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        filter: F,
        group_key_fn: GF,
        collector: C,
        weight_fn: W,
        is_hard: bool,
    ) -> Self {
        let node_name = constraint_ref.name.clone();
        let state = CrossGroupedNodeState::new(
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            filter,
            group_key_fn,
            collector,
        );
        let scorer =
            GroupedTerminalScorer::new(constraint_ref.clone(), impact_type, weight_fn, is_hard);
        Self {
            constraint_ref,
            is_hard,
            inner: SharedCrossGroupedConstraintSet::new(node_name, state, scorer),
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W2>(
        self,
        weight: W2,
    ) -> super::shared_set::CrossGroupedConstraintSetBuilder<
        S,
        A,
        B,
        JK,
        GK,
        EA,
        EB,
        KA,
        KB,
        F,
        GF,
        C,
        V,
        R,
        Acc,
        GroupedTerminalScorer<GK, R, W, Sc>,
        impl Fn(&GK, &R) -> Sc + Send + Sync,
        Sc,
    >
    where
        W2: for<'w> ConstraintWeight<(&'w GK, &'w R), Sc> + Send + Sync,
    {
        self.inner.penalize(weight)
    }

    pub fn reward<W2>(
        self,
        weight: W2,
    ) -> super::shared_set::CrossGroupedConstraintSetBuilder<
        S,
        A,
        B,
        JK,
        GK,
        EA,
        EB,
        KA,
        KB,
        F,
        GF,
        C,
        V,
        R,
        Acc,
        GroupedTerminalScorer<GK, R, W, Sc>,
        impl Fn(&GK, &R) -> Sc + Send + Sync,
        Sc,
    >
    where
        W2: for<'w> ConstraintWeight<(&'w GK, &'w R), Sc> + Send + Sync,
    {
        self.inner.reward(weight)
    }
}

impl<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc> IncrementalConstraint<S, Sc>
    for CrossGroupedConstraint<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    GK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + Send + Sync,
    EB: CollectionExtract<S, Item = B> + Send + Sync,
    KA: Fn(&A) -> JK + Send + Sync,
    KB: Fn(&B) -> JK + Send + Sync,
    F: Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
    GF: Fn(&A, &B) -> GK + Send + Sync,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    W: Fn(&GK, &R) -> Sc + Send + Sync,
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
