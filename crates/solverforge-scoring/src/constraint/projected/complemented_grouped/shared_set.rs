use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use crate::api::analysis::ConstraintAnalysis;
use crate::api::constraint_set::{ConstraintMetadata, ConstraintResult, ConstraintSet};
use crate::constraint::grouped::ComplementedGroupedScorerSet;
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::ProjectedSource;

use super::state::ProjectedComplementedGroupedNodeState;

pub struct SharedProjectedComplementedGroupedConstraintSet<
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
    Scorers,
    Sc,
> where
    Src: ProjectedSource<S, Out>,
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    state: ProjectedComplementedGroupedNodeState<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D>,
    scorers: Scorers,
    _phantom: PhantomData<fn() -> Sc>,
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, Scorers, Sc>
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
        Scorers,
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
    Scorers: ComplementedGroupedScorerSet<K, R, Sc>,
    Sc: Score + 'static,
{
    pub fn new(
        state: ProjectedComplementedGroupedNodeState<
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
        >,
        scorers: Scorers,
    ) -> Self {
        Self {
            state,
            scorers,
            _phantom: PhantomData,
        }
    }

    pub fn state(
        &self,
    ) -> &ProjectedComplementedGroupedNodeState<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D>
    {
        &self.state
    }
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, Scorers, Sc> ConstraintSet<S, Sc>
    for SharedProjectedComplementedGroupedConstraintSet<
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
        Scorers,
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
    Scorers: ComplementedGroupedScorerSet<K, R, Sc>,
    Sc: Score + 'static,
{
    fn evaluate_all(&self, solution: &S) -> Sc {
        let state = self.state.evaluation_state(solution);
        self.scorers.evaluate(&state)
    }

    fn constraint_count(&self) -> usize {
        self.scorers.constraint_count()
    }

    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        self.scorers.constraint_metadata()
    }

    fn evaluate_each<'a>(&'a self, solution: &S) -> Vec<ConstraintResult<'a, Sc>> {
        let state = self.state.evaluation_state(solution);
        self.scorers.evaluate_each(&state)
    }

    fn evaluate_detailed<'a>(&'a self, solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>> {
        let state = self.state.evaluation_state(solution);
        self.scorers.evaluate_detailed(&state)
    }

    fn initialize_all(&mut self, solution: &S) -> Sc {
        self.state.initialize(solution);
        self.scorers.initialize(&self.state)
    }

    fn on_insert_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.state
            .on_insert(solution, entity_index, descriptor_index);
        let changed_keys = self.state.take_changed_keys();
        self.scorers
            .refresh_changed_keys(&self.state, &changed_keys)
    }

    fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.state
            .on_retract(solution, entity_index, descriptor_index);
        let changed_keys = self.state.take_changed_keys();
        self.scorers
            .refresh_changed_keys(&self.state, &changed_keys)
    }

    fn reset_all(&mut self) {
        self.state.reset();
        self.scorers.reset();
    }
}
