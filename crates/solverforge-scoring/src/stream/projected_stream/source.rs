use std::hash::Hash;
use std::marker::PhantomData;

use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::filter::UniFilter;

pub trait ProjectionSink<Out> {
    fn emit(&mut self, output: Out);
}

pub trait Projection<A>: Send + Sync {
    type Out: Clone + Send + Sync + 'static;
    const MAX_EMITS: usize;

    fn project<Sink>(&self, input: &A, sink: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>;
}

struct VisitSink<V> {
    visit: V,
}

impl<Out, V> ProjectionSink<Out> for VisitSink<V>
where
    V: FnMut(Out),
{
    #[inline]
    fn emit(&mut self, output: Out) {
        (self.visit)(output);
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectedRowCoordinate {
    pub source_slot: usize,
    pub entity_index: usize,
    pub emit_index: usize,
}

#[doc(hidden)]
pub trait ProjectedSource<S, Out>: Send + Sync {
    const MAX_EMITS: usize;

    fn source_count(&self) -> usize;
    fn change_source(&self, slot: usize) -> ChangeSource;
    fn collect_all<V>(&self, solution: &S, visit: V)
    where
        V: FnMut(ProjectedRowCoordinate, Out);
    fn collect_entity<V>(&self, solution: &S, slot: usize, entity_index: usize, visit: V)
    where
        V: FnMut(ProjectedRowCoordinate, Out);
}

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
    Out: Clone + Send + Sync + 'static,
{
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

    fn collect_all<V>(&self, solution: &S, mut visit: V)
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
                    let coordinate = ProjectedRowCoordinate {
                        source_slot: 0,
                        entity_index: idx,
                        emit_index,
                    };
                    emit_index += 1;
                    visit(coordinate, output);
                },
            };
            self.projection.project(entity, &mut sink);
        }
    }

    fn collect_entity<V>(&self, solution: &S, slot: usize, entity_index: usize, mut visit: V)
    where
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
                let coordinate = ProjectedRowCoordinate {
                    source_slot: 0,
                    entity_index,
                    emit_index,
                };
                emit_index += 1;
                visit(coordinate, output);
            },
        };
        self.projection.project(entity, &mut sink);
    }
}

pub struct FilteredProjectedSource<S, Out, Src, F> {
    source: Src,
    filter: F,
    _phantom: PhantomData<(fn() -> S, fn() -> Out)>,
}

impl<S, Out, Src, F> FilteredProjectedSource<S, Out, Src, F> {
    pub(super) fn new(source: Src, filter: F) -> Self {
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
    Out: Clone + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
{
    const MAX_EMITS: usize = Src::MAX_EMITS;

    fn source_count(&self) -> usize {
        self.source.source_count()
    }

    fn change_source(&self, slot: usize) -> ChangeSource {
        self.source.change_source(slot)
    }

    fn collect_all<V>(&self, solution: &S, mut visit: V)
    where
        V: FnMut(ProjectedRowCoordinate, Out),
    {
        self.source.collect_all(solution, |coordinate, output| {
            if self.filter.test(solution, &output) {
                visit(coordinate, output);
            }
        });
    }

    fn collect_entity<V>(&self, solution: &S, slot: usize, entity_index: usize, mut visit: V)
    where
        V: FnMut(ProjectedRowCoordinate, Out),
    {
        self.source
            .collect_entity(solution, slot, entity_index, |coordinate, output| {
                if self.filter.test(solution, &output) {
                    visit(coordinate, output);
                }
            });
    }
}

pub struct MergedProjectedSource<Left, Right> {
    left: Left,
    right: Right,
}

impl<Left, Right> MergedProjectedSource<Left, Right> {
    pub(super) fn new(left: Left, right: Right) -> Self {
        Self { left, right }
    }
}

impl<S, Out, Left, Right> ProjectedSource<S, Out> for MergedProjectedSource<Left, Right>
where
    S: Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
    Left: ProjectedSource<S, Out>,
    Right: ProjectedSource<S, Out>,
{
    const MAX_EMITS: usize = Left::MAX_EMITS + Right::MAX_EMITS;

    fn source_count(&self) -> usize {
        self.left.source_count() + self.right.source_count()
    }

    fn change_source(&self, slot: usize) -> ChangeSource {
        let left_count = self.left.source_count();
        if slot < left_count {
            self.left.change_source(slot)
        } else {
            self.right.change_source(slot - left_count)
        }
    }

    fn collect_all<V>(&self, solution: &S, mut visit: V)
    where
        V: FnMut(ProjectedRowCoordinate, Out),
    {
        self.left.collect_all(solution, &mut visit);
        let left_count = self.left.source_count();
        self.right.collect_all(solution, |mut coordinate, output| {
            coordinate.source_slot += left_count;
            visit(coordinate, output);
        });
    }

    fn collect_entity<V>(&self, solution: &S, slot: usize, entity_index: usize, visit: V)
    where
        V: FnMut(ProjectedRowCoordinate, Out),
    {
        let left_count = self.left.source_count();
        if slot < left_count {
            self.left
                .collect_entity(solution, slot, entity_index, visit);
        } else {
            let mut visit = visit;
            self.right.collect_entity(
                solution,
                slot - left_count,
                entity_index,
                |mut coordinate, output| {
                    coordinate.source_slot += left_count;
                    visit(coordinate, output);
                },
            );
        }
    }
}
