use std::marker::PhantomData;

use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::filter::UniFilter;

use super::{ProjectedRowCoordinate, ProjectedSource, Projection, VisitSink};

pub struct SingleProjectedSource<S, A, E, F, P, Out> {
    extractor: E,
    filter: F,
    projection: P,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> Out)>,
}

impl<S, A, E, F, P, Out> SingleProjectedSource<S, A, E, F, P, Out> {
    pub(crate) fn new(extractor: E, filter: F, projection: P) -> Self {
        Self {
            extractor,
            filter,
            projection,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, E, F, P, Out> ProjectedSource<S, Out> for SingleProjectedSource<S, A, E, F, P, Out>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    P: Projection<A, Out = Out>,
    Out: Send + Sync + 'static,
{
    type State = ();

    const MAX_EMITS: usize = P::MAX_EMITS;

    fn source_count(&self) -> usize {
        1
    }

    fn change_source(&self, slot: usize) -> ChangeSource {
        if slot == 0 {
            self.extractor.change_source()
        } else {
            ChangeSource::Static
        }
    }

    fn build_state(&self, _solution: &S) -> Self::State {}

    fn collect_all<V>(&self, solution: &S, _state: &Self::State, mut visit: V)
    where
        V: FnMut(ProjectedRowCoordinate, Out),
    {
        for (idx, entity) in self.extractor.extract(solution).iter().enumerate() {
            if !self.filter.test(solution, entity) {
                continue;
            }
            let mut emit_index = 0;
            let mut sink = VisitSink {
                visit: |output| {
                    let coordinate = ProjectedRowCoordinate::single(0, idx, emit_index);
                    emit_index += 1;
                    visit(coordinate, output);
                },
            };
            self.projection.project(entity, &mut sink);
        }
    }

    fn collect_entity<V>(
        &self,
        solution: &S,
        _state: &Self::State,
        slot: usize,
        entity_index: usize,
        mut visit: V,
    ) where
        V: FnMut(ProjectedRowCoordinate, Out),
    {
        if slot != 0 {
            return;
        }
        let entities = self.extractor.extract(solution);
        let Some(entity) = entities.get(entity_index) else {
            return;
        };
        if !self.filter.test(solution, entity) {
            return;
        }
        let mut emit_index = 0;
        let mut sink = VisitSink {
            visit: |output| {
                let coordinate = ProjectedRowCoordinate::single(0, entity_index, emit_index);
                emit_index += 1;
                visit(coordinate, output);
            },
        };
        self.projection.project(entity, &mut sink);
    }

    fn insert_entity_state(
        &self,
        _solution: &S,
        _state: &mut Self::State,
        _slot: usize,
        _entity_index: usize,
    ) {
    }

    fn retract_entity_state(
        &self,
        _solution: &S,
        _state: &mut Self::State,
        _slot: usize,
        _entity_index: usize,
    ) {
    }
}
