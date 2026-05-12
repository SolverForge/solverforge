/* ListReverseMove - reverses a segment within a list variable.

This move reverses the order of elements in a range. Essential for
TSP 2-opt optimization where reversing a tour segment can reduce distance.

# Zero-Erasure Design

Uses concrete function pointers for list operations. No `dyn Any`, no downcasting.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::metadata::{
    encode_option_debug, encode_usize, scoped_move_identity, MoveTabuScope, ScopedValueTabuToken,
    TABU_OP_LIST_REVERSE,
};
use super::{Move, MoveTabuSignature};

/// A move that reverses a segment within a list.
///
/// This is the fundamental 2-opt move for TSP. Reversing a segment of the tour
/// can significantly reduce total distance by eliminating crossing edges.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The list element value type
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::ListReverseMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone, Debug)]
/// struct Tour { cities: Vec<i32>, score: Option<SoftScore> }
///
/// impl PlanningSolution for Tour {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn list_len(s: &Tour, _: usize) -> usize { s.cities.len() }
/// fn list_get(s: &Tour, _: usize, pos: usize) -> Option<i32> { s.cities.get(pos).copied() }
/// fn list_reverse(s: &mut Tour, _: usize, start: usize, end: usize) {
///     s.cities[start..end].reverse();
/// }
///
/// // Reverse segment [1..4) in tour: [A, B, C, D, E] -> [A, D, C, B, E]
/// let m = ListReverseMove::<Tour, i32>::new(
///     0, 1, 4,
///     list_len, list_get, list_reverse,
///     "cities", 0,
/// );
/// ```
pub struct ListReverseMove<S, V> {
    // Entity index
    entity_index: usize,
    // Start of range to reverse (inclusive)
    start: usize,
    // End of range to reverse (exclusive)
    end: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    // Reverse elements in range [start, end)
    list_reverse: fn(&mut S, usize, usize, usize),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> V>,
}

impl<S, V> Clone for ListReverseMove<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> Copy for ListReverseMove<S, V> {}

impl<S, V: Debug> Debug for ListReverseMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListReverseMove")
            .field("entity", &self.entity_index)
            .field("range", &(self.start..self.end))
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> ListReverseMove<S, V> {
    /* Creates a new list reverse move with concrete function pointers.

    # Arguments
    * `entity_index` - Entity index
    * `start` - Start of range (inclusive)
    * `end` - End of range (exclusive)
    * `list_len` - Function to get list length
    * `list_reverse` - Function to reverse elements in range
    * `variable_name` - Name of the list variable
    * `descriptor_index` - Entity descriptor index
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_index: usize,
        start: usize,
        end: usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_reverse: fn(&mut S, usize, usize, usize),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_index,
            start,
            end,
            list_len,
            list_get,
            list_reverse,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    pub fn entity_index(&self) -> usize {
        self.entity_index
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn segment_len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

impl<S, V> Move<S> for ListReverseMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Undo = ();

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();

        // Range must have at least 2 elements to be meaningful
        if self.end <= self.start + 1 {
            return false;
        }

        // Check range is within bounds
        let len = (self.list_len)(solution, self.entity_index);
        if self.end > len {
            return false;
        }

        true
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        // Notify before change
        score_director.before_variable_changed(self.descriptor_index, self.entity_index);

        // Reverse the segment
        (self.list_reverse)(
            score_director.working_solution_mut(),
            self.entity_index,
            self.start,
            self.end,
        );

        // Notify after change
        score_director.after_variable_changed(self.descriptor_index, self.entity_index);
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, (): Self::Undo) {
        self.do_move(score_director);
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        std::slice::from_ref(&self.entity_index)
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let mut value_ids: SmallVec<[u64; 2]> = SmallVec::new();
        for pos in self.start..self.end {
            let value = (self.list_get)(score_director.working_solution(), self.entity_index, pos);
            value_ids.push(encode_option_debug(value.as_ref()));
        }
        let entity_id = encode_usize(self.entity_index);
        let scope = MoveTabuScope::new(self.descriptor_index, self.variable_name);
        let destination_value_tokens: SmallVec<[ScopedValueTabuToken; 2]> = value_ids
            .iter()
            .copied()
            .map(|value_id| scope.value_token(value_id))
            .collect();
        let move_id = scoped_move_identity(
            scope,
            TABU_OP_LIST_REVERSE,
            [entity_id, encode_usize(self.start), encode_usize(self.end)],
        );

        MoveTabuSignature::new(scope, move_id.clone(), move_id)
            .with_entity_tokens([scope.entity_token(entity_id)])
            .with_destination_value_tokens(destination_value_tokens)
    }
}
