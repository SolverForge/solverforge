/* ListPermuteMove - permutes a contiguous window within a list variable. */

use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::metadata::{
    encode_option_debug, encode_usize, hash_str, MoveTabuScope, ScopedValueTabuToken,
    TABU_OP_LIST_PERMUTE,
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
}

impl<S, V> Move<S> for ListPermuteMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Undo = Vec<V>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();
        let len = (self.list_len)(solution, self.entity_index);
        self.start < self.end
            && self.end <= len
            && valid_non_identity_permutation(&self.permutation)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        score_director.before_variable_changed(self.descriptor_index, self.entity_index);
        let segment = (self.sublist_remove)(
            score_director.working_solution_mut(),
            self.entity_index,
            self.start,
            self.end,
        );
        let reordered = self
            .permutation
            .iter()
            .map(|&index| segment[index].clone())
            .collect();
        (self.sublist_insert)(
            score_director.working_solution_mut(),
            self.entity_index,
            self.start,
            reordered,
        );
        score_director.after_variable_changed(self.descriptor_index, self.entity_index);
        segment
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        score_director.before_variable_changed(self.descriptor_index, self.entity_index);
        let _ = (self.sublist_remove)(
            score_director.working_solution_mut(),
            self.entity_index,
            self.start,
            self.end,
        );
        (self.sublist_insert)(
            score_director.working_solution_mut(),
            self.entity_index,
            self.start,
            undo,
        );
        score_director.after_variable_changed(self.descriptor_index, self.entity_index);
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
        let scope = MoveTabuScope::new(self.descriptor_index, self.variable_name);
        let entity_id = encode_usize(self.entity_index);
        let variable_id = hash_str(self.variable_name);
        let mut touched_value_ids = SmallVec::<[u64; 8]>::new();
        for pos in self.start..self.end {
            let value = (self.list_get)(score_director.working_solution(), self.entity_index, pos);
            touched_value_ids.push(encode_option_debug(value.as_ref()));
        }

        let mut move_id = smallvec![
            TABU_OP_LIST_PERMUTE,
            encode_usize(self.descriptor_index),
            variable_id,
            entity_id,
            encode_usize(self.start),
            encode_usize(self.end),
        ];
        move_id.extend(self.permutation.iter().copied().map(encode_usize));
        move_id.extend(touched_value_ids.iter().copied());

        let mut undo_move_id = smallvec![
            TABU_OP_LIST_PERMUTE,
            encode_usize(self.descriptor_index),
            variable_id,
            entity_id,
            encode_usize(self.start),
            encode_usize(self.end),
        ];
        undo_move_id.extend(self.inverse_permutation.iter().copied().map(encode_usize));
        undo_move_id.extend(touched_value_ids.iter().copied());

        let destination_value_tokens: SmallVec<[ScopedValueTabuToken; 2]> = touched_value_ids
            .iter()
            .copied()
            .map(|value_id| scope.value_token(value_id))
            .collect();

        MoveTabuSignature::new(scope, move_id, undo_move_id)
            .with_entity_tokens([scope.entity_token(entity_id)])
            .with_destination_value_tokens(destination_value_tokens)
    }
}

fn valid_non_identity_permutation(permutation: &[usize]) -> bool {
    let len = permutation.len();
    if !(2..=MAX_LIST_PERMUTE_WINDOW_SIZE).contains(&len) {
        return false;
    }
    let mut seen = [false; MAX_LIST_PERMUTE_WINDOW_SIZE];
    let mut is_identity = true;
    for (idx, &value) in permutation.iter().enumerate() {
        if value >= len || seen[value] {
            return false;
        }
        seen[value] = true;
        is_identity &= value == idx;
    }
    !is_identity
}

fn inverse_permutation(permutation: &[usize]) -> SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]> {
    let mut inverse = smallvec![0; permutation.len()];
    for (new_idx, &old_idx) in permutation.iter().enumerate() {
        inverse[old_idx] = new_idx;
    }
    inverse
}
