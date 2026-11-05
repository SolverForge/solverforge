mod filtered;
mod joined;
mod merged;
mod operator;
mod single;

pub(super) use filtered::FilteredSource;
pub use joined::JoinedSource;
pub(super) use merged::MergedSource;
#[doc(hidden)]
pub(super) use operator::{ProjectedCollector, ProjectedRow, SourceNode};
pub(super) use single::SingleSource;

use crate::stream::collection_extract::ChangeSource;

/// Receives the rows a [`Projection`] emits.
pub trait ProjectionSink<Out> {
    /// Accepts one emitted row.
    fn emit(&mut self, output: Out);
}

/// Retained projection for
/// [`UniConstraintStream::project`](crate::stream::UniConstraintStream::project):
/// maps one entity to a bounded number of scoring rows.
pub trait Projection<A>: Send + Sync {
    /// The emitted row type; it need not be `Clone`.
    type Out: Send + Sync + 'static;
    /// Maximum number of rows one `project` call emits. Emitting more panics,
    /// naming the projection and both counts. Sources combine it (a merge adds
    /// both bounds).
    const MAX_EMITS: usize;

    /// Emits the rows for `input` into `sink`.
    fn project<Sink>(&self, input: &A, sink: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>;
}

/// Runs `projection` on `input`, passing each row with its emit index to
/// `visit`.
///
/// # Panics
///
/// When the projection emits more than `P::MAX_EMITS` rows.
fn project_bounded<A, P, V>(projection: &P, input: &A, visit: V)
where
    P: Projection<A>,
    V: FnMut(usize, P::Out),
{
    let mut sink = BoundedSink {
        visit,
        emitted: 0,
        max_emits: P::MAX_EMITS,
        projection: std::any::type_name::<P>,
    };
    projection.project(input, &mut sink);
}

struct BoundedSink<V> {
    visit: V,
    emitted: usize,
    max_emits: usize,
    projection: fn() -> &'static str,
}

impl<Out, V> ProjectionSink<Out> for BoundedSink<V>
where
    V: FnMut(usize, Out),
{
    #[inline]
    fn emit(&mut self, output: Out) {
        if self.emitted == self.max_emits {
            emitted_past_bound((self.projection)(), self.max_emits);
        }
        let emit_index = self.emitted;
        self.emitted += 1;
        (self.visit)(emit_index, output);
    }
}

#[cold]
#[inline(never)]
fn emitted_past_bound(projection: &str, max_emits: usize) -> ! {
    panic!(
        "projection `{projection}` emitted {} rows for one input; its MAX_EMITS is {max_emits}",
        max_emits + 1
    );
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
