use std::marker::PhantomData;

use crate::stream::collection_extract::ChangeSource;
use crate::stream::filter::UniFilter;

use super::{ProjectedRowCoordinate, ProjectedSource};

pub struct FilteredProjectedSource<S, Out, Src, F> {
    source: Src,
    filter: F,
    _phantom: PhantomData<(fn() -> S, fn() -> Out)>,
}

impl<S, Out, Src, F> FilteredProjectedSource<S, Out, Src, F> {
    pub(crate) fn new(source: Src, filter: F) -> Self {
        Self {
            source,
            filter,
            _phantom: PhantomData,
        }
    }
}

impl<S, Out, Src, F> ProjectedSource<S, Out> for FilteredProjectedSource<S, Out, Src, F>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
{
    type State = Src::State;

    const MAX_EMITS: usize = Src::MAX_EMITS;

    fn source_count(&self) -> usize {
        self.source.source_count()
    }

    fn change_source(&self, slot: usize) -> ChangeSource {
        self.source.change_source(slot)
    }

    fn build_state(&self, solution: &S) -> Self::State {
        self.source.build_state(solution)
    }

    fn collect_all<V>(&self, solution: &S, state: &Self::State, mut visit: V)
    where
        V: FnMut(ProjectedRowCoordinate, Out),
    {
        self.source
            .collect_all(solution, state, |coordinate, output| {
                if self.filter.test(solution, &output) {
                    visit(coordinate, output);
                }
            });
    }

    fn collect_entity<V>(
        &self,
        solution: &S,
        state: &Self::State,
        slot: usize,
        entity_index: usize,
        mut visit: V,
    ) where
        V: FnMut(ProjectedRowCoordinate, Out),
    {
        self.source
            .collect_entity(solution, state, slot, entity_index, |coordinate, output| {
                if self.filter.test(solution, &output) {
                    visit(coordinate, output);
                }
            });
    }

    fn insert_entity_state(
        &self,
        solution: &S,
        state: &mut Self::State,
        slot: usize,
        entity_index: usize,
    ) {
        self.source
            .insert_entity_state(solution, state, slot, entity_index);
    }

    fn retract_entity_state(
        &self,
        solution: &S,
        state: &mut Self::State,
        slot: usize,
        entity_index: usize,
    ) {
        self.source
            .retract_entity_state(solution, state, slot, entity_index);
    }
}
