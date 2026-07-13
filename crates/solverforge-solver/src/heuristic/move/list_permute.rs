/* ListPermuteMove - permutes a contiguous window within a list variable. */

use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::list_kernel::{
    permute_candidate_trace_identity, permute_do_move, permute_is_doable, permute_tabu_signature,
    permute_undo_move, PermuteCoordinates, StaticListWindowAccess,
};
use super::{Move, MoveTabuSignature};

pub const MAX_LIST_PERMUTE_WINDOW_SIZE: usize = 8;

pub struct ListPermuteMove<S, V> {
    entity_index: usize,
    start: usize,
    end: usize,
    permutation: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]>,
    inverse_permutation: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]>,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    entity_indices: [usize; 1],
    _phantom: PhantomData<fn() -> V>,
}

impl<S, V> Clone for ListPermuteMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            entity_index: self.entity_index,
            start: self.start,
            end: self.end,
            permutation: self.permutation.clone(),
            inverse_permutation: self.inverse_permutation.clone(),
            list_len: self.list_len,
            list_get: self.list_get,
            sublist_remove: self.sublist_remove,
            sublist_insert: self.sublist_insert,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            entity_indices: self.entity_indices,
            _phantom: PhantomData,
        }
    }
}

impl<S, V: Debug> Debug for ListPermuteMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListPermuteMove")
            .field("entity", &self.entity_index)
            .field("start", &self.start)
            .field("end", &self.end)
            .field("permutation", &self.permutation)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> ListPermuteMove<S, V> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_index: usize,
        start: usize,
        end: usize,
        permutation: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]>,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        let window_len = end.saturating_sub(start);
        assert!(
            (2..=MAX_LIST_PERMUTE_WINDOW_SIZE).contains(&window_len),
            "list permute window length must be 2..={MAX_LIST_PERMUTE_WINDOW_SIZE}",
        );
        assert_eq!(
            permutation.len(),
            window_len,
            "list permute permutation length must match window length",
        );
        let inverse_permutation = inverse_permutation(&permutation);
        Self {
            entity_index,
            start,
            end,
            permutation,
            inverse_permutation,
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            entity_indices: [entity_index],
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn entity_index(&self) -> usize {
        self.entity_index
    }

    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }

    #[inline]
    pub fn permutation(&self) -> &[usize] {
        &self.permutation
    }

    fn access(&self) -> StaticListWindowAccess<S, V> {
        StaticListWindowAccess {
            list_len: self.list_len,
            list_get: self.list_get,
            sublist_remove: self.sublist_remove,
            sublist_insert: self.sublist_insert,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
        }
    }

    fn coordinates(&self) -> PermuteCoordinates {
        PermuteCoordinates {
            entity: self.entity_index,
            start: self.start,
            end: self.end,
        }
    }
}

impl<S, V> Move<S> for ListPermuteMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Undo = Vec<V>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        permute_is_doable(
            &self.access(),
            self.coordinates(),
            &self.permutation,
            score_director,
        )
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        permute_do_move(
            &self.access(),
            self.coordinates(),
            &self.permutation,
            score_director,
        )
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        permute_undo_move(&self.access(), self.coordinates(), undo, score_director);
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }

    fn telemetry_label(&self) -> &'static str {
        "list_permute"
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        permute_tabu_signature(
            &self.access(),
            self.coordinates(),
            &self.permutation,
            &self.inverse_permutation,
            score_director,
        )
    }

    fn candidate_trace_identity(&self) -> Option<crate::stats::CandidateTraceIdentity> {
        Some(permute_candidate_trace_identity(
            &self.access(),
            self.coordinates(),
            &self.permutation,
        ))
    }
}

fn inverse_permutation(permutation: &[usize]) -> SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]> {
    let mut inverse = smallvec![0; permutation.len()];
    for (new_idx, &old_idx) in permutation.iter().enumerate() {
        inverse[old_idx] = new_idx;
    }
    inverse
}
