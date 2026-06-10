mod filtered;
mod joined;
mod merged;
mod single;

pub use filtered::FilteredSource;
pub use joined::JoinedSource;
pub use merged::MergedSource;
pub use single::SingleSource;

use crate::stream::collection_extract::ChangeSource;

pub trait ProjectionSink<Out> {
    fn emit(&mut self, output: Out);
}

pub trait Projection<A>: Send + Sync {
    type Out: Send + Sync + 'static;
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
pub struct RowOwner {
    pub source_slot: usize,
    pub entity_index: usize,
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowCoordinate {
    pub primary_owner: RowOwner,
    pub secondary_owner: Option<RowOwner>,
    pub emit_index: usize,
}

impl RowCoordinate {
    #[inline]
    pub fn single(source_slot: usize, entity_index: usize, emit_index: usize) -> Self {
        Self {
            primary_owner: RowOwner {
                source_slot,
                entity_index,
            },
            secondary_owner: None,
            emit_index,
        }
    }

    #[inline]
    pub fn pair(
        left_slot: usize,
        left_index: usize,
        right_slot: usize,
        right_index: usize,
        emit_index: usize,
    ) -> Self {
        Self {
            primary_owner: RowOwner {
                source_slot: left_slot,
                entity_index: left_index,
            },
            secondary_owner: Some(RowOwner {
                source_slot: right_slot,
                entity_index: right_index,
            }),
            emit_index,
        }
    }

    #[inline]
    pub fn for_each_owner<V>(&self, mut visit: V)
    where
        V: FnMut(RowOwner),
    {
        visit(self.primary_owner);
        if let Some(owner) = self.secondary_owner {
            if owner != self.primary_owner {
                visit(owner);
            }
        }
    }

    #[inline]
    pub fn offset_source_slots(mut self, offset: usize) -> Self {
        self.primary_owner.source_slot += offset;
        if let Some(owner) = self.secondary_owner.as_mut() {
            owner.source_slot += offset;
        }
        self
    }
}

#[doc(hidden)]
pub trait Source<S, Out>: Send + Sync {
    type State: Send + Sync;

    const MAX_EMITS: usize;

    fn source_count(&self) -> usize;
    fn change_source(&self, slot: usize) -> ChangeSource;
    fn build_state(&self, solution: &S) -> Self::State;
    fn collect_all<V>(&self, solution: &S, state: &Self::State, visit: V)
    where
        V: FnMut(RowCoordinate, Out);
    fn collect_entity<V>(
        &self,
        solution: &S,
        state: &Self::State,
        slot: usize,
        entity_index: usize,
        visit: V,
    ) where
        V: FnMut(RowCoordinate, Out);
    fn insert_entity_state(
        &self,
        solution: &S,
        state: &mut Self::State,
        slot: usize,
        entity_index: usize,
    );
    fn retract_entity_state(
        &self,
        solution: &S,
        state: &mut Self::State,
        slot: usize,
        entity_index: usize,
    );
}
