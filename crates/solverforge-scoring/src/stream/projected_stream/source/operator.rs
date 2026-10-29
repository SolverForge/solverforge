use std::marker::PhantomData;

use crate::api::analysis::EntityRef;
use crate::stream::{
    collector::{Accumulator, Collector},
    relational::{
        hash::{FastMap, FastSet},
        operator::{ExplainRow, Operator},
        DenseRowStore, RowHandle,
    },
};

use super::{RowCoordinate, RowOwner, Source};

#[doc(hidden)]
pub struct ProjectedRow<'a, Out> {
    pub(crate) value: &'a Out,
}

impl<Out> Copy for ProjectedRow<'_, Out> {}
impl<Out> Clone for ProjectedRow<'_, Out> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<Out> ExplainRow for ProjectedRow<'_, Out> {
    fn explain(&self, _entities: &mut Vec<EntityRef>) {}
}

struct StoredRow<Out> {
    coordinate: RowCoordinate,
    value: Out,
}

#[doc(hidden)]
pub struct SourceNode<S, Out, Src>
where
    Src: Source<S, Out>,
{
    source: Src,
    source_state: Option<Src::State>,
    rows: DenseRowStore<StoredRow<Out>>,
    by_coordinate: FastMap<RowCoordinate, RowHandle>,
    by_owner: FastMap<RowOwner, Vec<RowHandle>>,
    marker: PhantomData<fn() -> S>,
}

impl<S, Out, Src> SourceNode<S, Out, Src>
where
    Src: Source<S, Out>,
{
    pub(crate) fn new(source: Src) -> Self {
        Self {
            source,
            source_state: None,
            rows: DenseRowStore::new(),
            by_coordinate: FastMap::default(),
            by_owner: FastMap::default(),
            marker: PhantomData,
        }
    }

    fn insert_row(&mut self, coordinate: RowCoordinate, value: Out) -> Option<RowHandle> {
        if self.by_coordinate.contains_key(&coordinate) {
            return None;
        }
        let handle = self.rows.insert(StoredRow { coordinate, value });
        self.by_coordinate.insert(coordinate, handle);
        coordinate.for_each_owner(|owner| self.by_owner.entry(owner).or_default().push(handle));
        Some(handle)
    }

    fn remove_handle(&mut self, handle: RowHandle) -> bool {
        let Some(row) = self.rows.retract(handle) else {
            return false;
        };
        self.by_coordinate.remove(&row.coordinate);
        row.coordinate.for_each_owner(|owner| {
            let remove_bucket = if let Some(handles) = self.by_owner.get_mut(&owner) {
                handles.retain(|candidate| *candidate != handle);
                handles.is_empty()
            } else {
                false
            };
            if remove_bucket {
                self.by_owner.remove(&owner);
            }
        });
        true
    }

    fn localized_owners(&self, descriptor: usize, index: usize) -> Vec<RowOwner> {
        (0..self.source.source_count())
            .filter(|slot| self.source.change_source(*slot).owns_descriptor(descriptor))
            .map(|source_slot| RowOwner {
                source_slot,
                entity_index: index,
            })
            .collect()
    }
}

impl<S, Out, Src> Operator<S> for SourceNode<S, Out, Src>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    Src: Source<S, Out> + 'static,
{
    type View<'a> = ProjectedRow<'a, Out>;
    type Evaluation = (Src::State, Vec<(RowCoordinate, Out)>);

    fn prepare_evaluation(&self, solution: &S) -> Self::Evaluation {
        let state = self.source.build_state(solution);
        let mut rows = Vec::new();
        self.source
            .collect_all(solution, &state, |coordinate, value| {
                rows.push((coordinate, value))
            });
        (state, rows)
    }

    fn visit_evaluation<'a>(
        &'a self,
        _solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        for (coordinate, value) in &evaluation.1 {
            let _ = coordinate;
            visitor(ProjectedRow { value });
        }
    }

    fn clear(&mut self) {
        self.source_state = None;
        self.rows.clear();
        self.by_coordinate.clear();
        self.by_owner.clear();
    }

    fn initialize(&mut self, solution: &S) {
        self.clear();
        let state = self.source.build_state(solution);
        let mut rows = Vec::new();
        self.source
            .collect_all(solution, &state, |coordinate, value| {
                rows.push((coordinate, value))
            });
        self.source_state = Some(state);
        for (coordinate, value) in rows {
            self.insert_row(coordinate, value);
        }
    }

    fn handles(&self) -> Vec<RowHandle> {
        self.rows.iter().map(|(handle, _)| handle).collect()
    }

    fn resolve<'a>(&'a self, _solution: &'a S, handle: RowHandle) -> Option<Self::View<'a>> {
        self.rows
            .get(handle)
            .map(|row| ProjectedRow { value: &row.value })
    }

    fn visit_provenance(&self, handle: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        let Some(row) = self.rows.get(handle) else {
            return;
        };
        row.coordinate.for_each_owner(|owner| {
            if let crate::stream::collection_extract::ChangeSource::Descriptor(descriptor) =
                self.source.change_source(owner.source_slot)
            {
                visitor(owner.source_slot as u32, descriptor, owner.entity_index);
            }
        });
    }

    fn retract(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        removed.clear();
        inserted.clear();
        let owners = self.localized_owners(descriptor, index);
        let mut seen = FastSet::default();
        let handles = owners
            .iter()
            .flat_map(|owner| self.by_owner.get(owner).into_iter().flatten().copied())
            .filter(|handle| seen.insert(*handle))
            .collect::<Vec<_>>();
        for handle in handles {
            if self.remove_handle(handle) {
                removed.push(handle);
            }
        }
        if let Some(state) = self.source_state.as_mut() {
            for owner in owners {
                self.source.retract_entity_state(
                    solution,
                    state,
                    owner.source_slot,
                    owner.entity_index,
                );
            }
        }
    }

    fn insert(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        removed.clear();
        inserted.clear();
        let owners = self.localized_owners(descriptor, index);
        if owners.is_empty() {
            return;
        }
        if self.source_state.is_none() {
            self.source_state = Some(self.source.build_state(solution));
        }
        let state = self.source_state.as_mut().expect("projected source state");
        for owner in &owners {
            self.source
                .insert_entity_state(solution, state, owner.source_slot, owner.entity_index);
        }
        let state = self.source_state.as_ref().expect("projected source state");
        let mut rows = Vec::new();
        for owner in owners {
            self.source.collect_entity(
                solution,
                state,
                owner.source_slot,
                owner.entity_index,
                |coordinate, value| rows.push((coordinate, value)),
            );
        }
        for (coordinate, value) in rows {
            if let Some(handle) = self.insert_row(coordinate, value) {
                inserted.push(handle);
            }
        }
    }
}

#[doc(hidden)]
pub struct ProjectedCollector<C, V, R, Acc> {
    inner: C,
    marker: PhantomData<fn() -> (V, R, Acc)>,
}

impl<C, V, R, Acc> ProjectedCollector<C, V, R, Acc> {
    pub(crate) fn new(inner: C) -> Self {
        Self {
            inner,
            marker: PhantomData,
        }
    }
}

impl<Out, C, V, R, Acc> Collector<ProjectedRow<'_, Out>> for ProjectedCollector<C, V, R, Acc>
where
    C: for<'a> Collector<&'a Out, Value = V, Result = R, Accumulator = Acc>,
    Acc: Accumulator<V, R>,
    R: Send + Sync,
{
    type Value = V;
    type Result = R;
    type Accumulator = Acc;
    fn extract(&self, input: ProjectedRow<'_, Out>) -> V {
        self.inner.extract(input.value)
    }
    fn create_accumulator(&self) -> Acc {
        self.inner.create_accumulator()
    }
}
