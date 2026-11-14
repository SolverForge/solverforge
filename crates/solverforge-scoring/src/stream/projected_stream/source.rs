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

use crate::api::analysis::EntityRef;
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
    /// naming the projection and both counts. Retained scoring state sizes
    /// each entity's row list by it, and sources combine it (a merge adds both
    /// bounds).
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

/// One entity a projected row is derived from: which of the source's
/// collections it lives in and its index there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowOwner {
    /// The source collection slot, `0..Source::source_count()`.
    pub source_slot: usize,
    /// The entity's index in that collection.
    pub entity_index: usize,
}

/// The stable identity of one projected row: the entity or pair of entities
/// it comes from and its position among their emissions. Ordering is by
/// coordinate, which keeps projected self-joins independent of storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowCoordinate {
    /// The owner the row's index-based filters see.
    pub primary_owner: RowOwner,
    /// The second owner of a row emitted for a joined pair.
    pub secondary_owner: Option<RowOwner>,
    /// The row's position among the rows emitted for its owners.
    pub emit_index: usize,
}

impl RowCoordinate {
    /// A row emitted for one entity.
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

    /// A row emitted for a joined pair of entities.
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

    /// Visits each distinct owner, primary first.
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

    /// Shifts every owner's source slot by `offset`, as a merge does for its
    /// right input.
    #[inline]
    pub fn offset_source_slots(mut self, offset: usize) -> Self {
        self.primary_owner.source_slot += offset;
        if let Some(owner) = self.secondary_owner.as_mut() {
            owner.source_slot += offset;
        }
        self
    }
}

/// The producer behind a projected stream: one or more entity collections
/// and the rows they project to.
///
/// A projected stream's operator tree retains rows by [`RowCoordinate`] and,
/// on an entity notification, asks the source for that entity's rows again.
/// Single-source projections, filters, merges and joined pairs implement it;
/// the fluent `project`, `filter`, `merge` and `join` methods build them.
pub trait Source<S, Out>: Send + Sync {
    /// State the source keeps beside the retained rows, such as a join index.
    type State: Send + Sync;

    /// The most rows the source emits for one owner (see
    /// [`Projection::MAX_EMITS`]).
    const MAX_EMITS: usize;

    /// Number of entity collections the source reads.
    fn source_count(&self) -> usize;
    /// How changes reach collection `slot`.
    fn change_source(&self, slot: usize) -> ChangeSource;
    /// Builds the source state for `solution`.
    fn build_state(&self, solution: &S) -> Self::State;
    /// Visits every row of `solution` with its coordinate.
    fn collect_all<V>(&self, solution: &S, state: &Self::State, visit: V)
    where
        V: FnMut(RowCoordinate, Out);
    /// Visits the rows owned by entity `entity_index` of collection `slot`.
    fn collect_entity<V>(
        &self,
        solution: &S,
        state: &Self::State,
        slot: usize,
        entity_index: usize,
        visit: V,
    ) where
        V: FnMut(RowCoordinate, Out);
    /// Adds entity `entity_index` of collection `slot` to the source state.
    fn insert_entity_state(
        &self,
        solution: &S,
        state: &mut Self::State,
        slot: usize,
        entity_index: usize,
    );
    /// Removes entity `entity_index` of collection `slot` from the source
    /// state.
    fn retract_entity_state(
        &self,
        solution: &S,
        state: &mut Self::State,
        slot: usize,
        entity_index: usize,
    );
}

/// A [`Source`] that can name the entity behind each owner of its rows, for
/// score explanation. Read only while explaining a match, never while
/// scoring.
pub trait ExplainSource<S, Out>: Source<S, Out> {
    /// Appends the entity `owner` names, with its planning id when its
    /// collection reports one, to `entities`.
    fn explain_owner(&self, solution: &S, owner: RowOwner, entities: &mut Vec<EntityRef>);
}
