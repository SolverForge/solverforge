/* ListMultiSwapMove - applies multiple independent intra-list swaps.

This move coordinates several swaps across distinct list entities while keeping
the same concrete function-pointer access pattern as the existing list moves.
*/

use std::fmt::Debug;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::metadata::{
    encode_option_debug, encode_usize, scoped_move_identity, MoveTabuScope, ScopedEntityTabuToken,
    ScopedValueTabuToken, TABU_OP_LIST_MULTI_SWAP,
};
use super::{Move, MoveTabuSignature};

/// A move that applies multiple independent intra-list swaps.
///
/// Each swap is encoded as `(entity_index, first_position, second_position)`.
/// Swaps must target distinct entities; this keeps notification and undo
/// semantics simple and avoids order-dependent overlap within one list.
pub struct ListMultiSwapMove<S, V> {
    swaps: SmallVec<[(usize, usize, usize); 4]>,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_set: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
    indices: SmallVec<[usize; 4]>,
    require_score_improvement: bool,
}

impl<S, V> Clone for ListMultiSwapMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            swaps: self.swaps.clone(),
            list_len: self.list_len,
            list_get: self.list_get,
            list_set: self.list_set,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            indices: self.indices.clone(),
            require_score_improvement: self.require_score_improvement,
        }
    }
}

impl<S, V: Debug> Debug for ListMultiSwapMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListMultiSwapMove")
            .field("swaps", &self.swaps)
            .field("variable_name", &self.variable_name)
            .field("require_score_improvement", &self.require_score_improvement)
            .finish()
    }
}

impl<S, V> ListMultiSwapMove<S, V> {
    pub fn new(
        swaps: &[(usize, usize, usize)],
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        let mut indices = SmallVec::new();
        for &(entity, _, _) in swaps {
            if !indices.contains(&entity) {
                indices.push(entity);
            }
        }
        Self {
            swaps: swaps.iter().copied().collect(),
            list_len,
            list_get,
            list_set,
            variable_name,
            descriptor_index,
            indices,
            require_score_improvement: false,
        }
    }

    pub fn with_require_score_improvement(mut self, require_score_improvement: bool) -> Self {
        self.require_score_improvement = require_score_improvement;
        self
    }

    pub fn swaps(&self) -> &[(usize, usize, usize)] {
        &self.swaps
    }
}

impl<S, V> Move<S> for ListMultiSwapMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Undo = ();

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        if self.swaps.is_empty() {
            return false;
        }

        let solution = score_director.working_solution();
        let mut seen_entities = SmallVec::<[usize; 4]>::new();
        for &(entity, first, second) in &self.swaps {
            if first == second || seen_entities.contains(&entity) {
                return false;
            }
            seen_entities.push(entity);

            let len = (self.list_len)(solution, entity);
            if first >= len || second >= len {
                return false;
            }

            let first_val = (self.list_get)(solution, entity, first);
            let second_val = (self.list_get)(solution, entity, second);
            if first_val != second_val {
                continue;
            }
            return false;
        }

        true
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        let mut values = SmallVec::<[(V, V); 4]>::new();
        for &(entity, first, second) in &self.swaps {
            let first_val = (self.list_get)(score_director.working_solution(), entity, first)
                .expect("first position should be valid");
            let second_val = (self.list_get)(score_director.working_solution(), entity, second)
                .expect("second position should be valid");
            values.push((first_val, second_val));
        }

        for &entity in &self.indices {
            score_director.before_variable_changed(self.descriptor_index, entity);
        }

        for (&(entity, first, second), (first_val, second_val)) in
            self.swaps.iter().zip(values.iter())
        {
            (self.list_set)(
                score_director.working_solution_mut(),
                entity,
                first,
                second_val.clone(),
            );
            (self.list_set)(
                score_director.working_solution_mut(),
                entity,
                second,
                first_val.clone(),
            );
        }

        for &entity in &self.indices {
            score_director.after_variable_changed(self.descriptor_index, entity);
        }
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, (): Self::Undo) {
        self.do_move(score_director);
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.indices
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }

    fn telemetry_label(&self) -> &'static str {
        "list_multi_swap"
    }

    fn requires_score_improvement(&self) -> bool {
        self.require_score_improvement
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let scope = MoveTabuScope::new(self.descriptor_index, self.variable_name);
        let mut entity_tokens = SmallVec::<[ScopedEntityTabuToken; 2]>::new();
        for &entity in &self.indices {
            entity_tokens.push(scope.entity_token(encode_usize(entity)));
        }

        let mut destination_value_tokens = SmallVec::<[ScopedValueTabuToken; 2]>::new();
        for &(entity, first, second) in &self.swaps {
            let first_val = (self.list_get)(score_director.working_solution(), entity, first);
            let second_val = (self.list_get)(score_director.working_solution(), entity, second);
            destination_value_tokens
                .push(scope.value_token(encode_option_debug(second_val.as_ref())));
            destination_value_tokens
                .push(scope.value_token(encode_option_debug(first_val.as_ref())));
        }

        let mut canonical = self
            .swaps
            .iter()
            .map(|&(entity, first, second)| {
                let (left, right) = if first <= second {
                    (first, second)
                } else {
                    (second, first)
                };
                (entity, left, right)
            })
            .collect::<SmallVec<[(usize, usize, usize); 4]>>();
        canonical.sort_unstable();

        let mut components = SmallVec::<[u64; 8]>::new();
        components.push(encode_usize(canonical.len()));
        for (entity, first, second) in canonical {
            components.push(encode_usize(entity));
            components.push(encode_usize(first));
            components.push(encode_usize(second));
        }
        let move_id = scoped_move_identity(scope, TABU_OP_LIST_MULTI_SWAP, components);

        MoveTabuSignature::new(scope, move_id.clone(), move_id)
            .with_entity_tokens(entity_tokens)
            .with_destination_value_tokens(destination_value_tokens)
    }
}
