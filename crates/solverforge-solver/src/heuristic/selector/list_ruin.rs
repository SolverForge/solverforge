//! Public static adapter for the canonical streamed list-ruin cursor.

use std::cell::RefCell;
use std::fmt::Debug;
use std::marker::PhantomData;

use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::ListRuinMove;
use crate::heuristic::selector::list_kernel::{NativeRuinEmitter, RuinCursor, RuinSourcePool};

use super::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

/// Public list ruin selector. Candidate source selection is shared; the
/// emitted move retains its established self-contained recreate behavior.
pub struct ListRuinMoveSelector<S, V> {
    min_ruin_count: usize,
    max_ruin_count: usize,
    rng: RefCell<SmallRng>,
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_remove: fn(&mut S, usize, usize) -> V,
    list_insert: fn(&mut S, usize, usize, V),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    variable_name: &'static str,
    descriptor_index: usize,
    moves_per_step: usize,
    max_source_list_len: Option<usize>,
    skip_empty_destinations: bool,
    _phantom: PhantomData<fn() -> V>,
}

// The RefCell is touched only while opening a cursor to derive its private
// seed; the cursor owns its RNG after that boundary.
unsafe impl<S, V> Send for ListRuinMoveSelector<S, V> {}

impl<S, V: Debug> Debug for ListRuinMoveSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListRuinMoveSelector")
            .field("min_ruin_count", &self.min_ruin_count)
            .field("max_ruin_count", &self.max_ruin_count)
            .field("moves_per_step", &self.moves_per_step)
            .field("max_source_list_len", &self.max_source_list_len)
            .field("skip_empty_destinations", &self.skip_empty_destinations)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

pub struct ListRuinMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    inner: RuinCursor<S, NativeRuinEmitter<S, V>>,
}

impl<S, V> ListRuinMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn new(inner: RuinCursor<S, NativeRuinEmitter<S, V>>) -> Self {
        Self { inner }
    }
}

impl<S, V> MoveCursor<S, ListRuinMove<S, V>> for ListRuinMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.inner.next_candidate()
    }
    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ListRuinMove<S, V>>> {
        self.inner.candidate(id)
    }
    fn take_candidate(&mut self, id: CandidateId) -> ListRuinMove<S, V> {
        self.inner.take_candidate(id)
    }
    fn next_owned_candidate(&mut self) -> Option<ListRuinMove<S, V>> {
        self.inner.next_owned_candidate()
    }
    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'a> fn(MoveCandidateRef<'a, S, ListRuinMove<S, V>>) -> bool,
    ) -> Option<ListRuinMove<S, V>> {
        self.inner.next_owned_candidate_matching(predicate)
    }
    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }
}

impl<S, V> Iterator for ListRuinMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Item = ListRuinMove<S, V>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

impl<S, V> ListRuinMoveSelector<S, V> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        min_ruin_count: usize,
        max_ruin_count: usize,
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> V,
        list_insert: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        assert!(min_ruin_count >= 1, "min_ruin_count must be at least 1");
        assert!(
            max_ruin_count >= min_ruin_count,
            "max_ruin_count must be >= min_ruin_count"
        );
        Self {
            min_ruin_count,
            max_ruin_count,
            rng: RefCell::new(SmallRng::from_rng(&mut rand::rng())),
            entity_count,
            list_len,
            list_get,
            list_remove,
            list_insert,
            element_owner_fn: None,
            variable_name,
            descriptor_index,
            moves_per_step: 10,
            max_source_list_len: None,
            skip_empty_destinations: false,
            _phantom: PhantomData,
        }
    }

    pub fn with_moves_per_step(mut self, count: usize) -> Self {
        self.moves_per_step = count;
        self
    }
    pub fn with_max_source_list_len(mut self, max_source_list_len: Option<usize>) -> Self {
        self.max_source_list_len = max_source_list_len;
        self
    }
    pub fn with_skip_empty_destinations(mut self, skip_empty_destinations: bool) -> Self {
        self.skip_empty_destinations = skip_empty_destinations;
        self
    }
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = RefCell::new(SmallRng::seed_from_u64(seed));
        self
    }
    pub fn with_element_owner_fn(
        mut self,
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    ) -> Self {
        self.element_owner_fn = element_owner_fn;
        self
    }
}

impl<S, V> MoveSelector<S, ListRuinMove<S, V>> for ListRuinMoveSelector<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Cursor<'a>
        = ListRuinMoveCursor<S, V>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        let solution = score_director.working_solution();
        let entity_count = (self.entity_count)(solution);
        let non_empty = (0..entity_count)
            .filter_map(|entity| {
                let len = (self.list_len)(solution, entity);
                (len > 0
                    && self
                        .max_source_list_len
                        .is_none_or(|max_len| len <= max_len))
                .then_some((entity, len))
            })
            .collect::<Vec<_>>();
        let source_pool = if let Some(owner_fn) = self.element_owner_fn {
            let eligible = non_empty
                .iter()
                .filter_map(|&(entity, len)| {
                    let mut positions = SmallVec::new();
                    for position in 0..len {
                        let Some(element) = (self.list_get)(solution, entity, position) else {
                            continue;
                        };
                        if crate::list_placement::candidate_entity_indices(
                            Some(owner_fn),
                            solution,
                            entity_count,
                            &element,
                        )
                        .next()
                        .is_some()
                        {
                            positions.push(position);
                        }
                    }
                    (!positions.is_empty()).then_some((entity, positions))
                })
                .collect();
            RuinSourcePool::OwnerRestricted(eligible)
        } else {
            RuinSourcePool::Unrestricted(non_empty)
        };
        let seed = self.rng.borrow_mut().random::<u64>()
            ^ context.offset_seed(0x7157_8011_C0DE_0001) as u64;
        ListRuinMoveCursor::new(RuinCursor::new(
            NativeRuinEmitter::new(
                self.entity_count,
                self.list_len,
                self.list_get,
                self.list_remove,
                self.list_insert,
                self.element_owner_fn,
                self.skip_empty_destinations,
                self.variable_name,
                self.descriptor_index,
            ),
            SmallRng::seed_from_u64(seed),
            source_pool,
            self.moves_per_step,
            self.min_ruin_count,
            self.max_ruin_count,
        ))
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        if (self.entity_count)(score_director.working_solution()) > 0 {
            self.moves_per_step
        } else {
            0
        }
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}
