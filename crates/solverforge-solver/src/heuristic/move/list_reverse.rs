/* ListReverseMove - reverses a segment within a list variable.

This move reverses the order of elements in a range. Essential for
TSP 2-opt optimization where reversing a tour segment can reduce distance.

# Zero-Erasure Design

Uses concrete function pointers for list operations. No `dyn Any`, no downcasting.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::list_kernel::{
    reverse_candidate_trace_identity, reverse_do_move, reverse_is_doable, reverse_tabu_signature,
    ReverseCoordinates, StaticListReverseAccess,
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

    fn access(&self) -> StaticListReverseAccess<S, V> {
        StaticListReverseAccess {
            list_len: self.list_len,
            list_get: self.list_get,
            list_reverse: self.list_reverse,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
        }
    }

    fn coordinates(&self) -> ReverseCoordinates {
        ReverseCoordinates {
            entity: self.entity_index,
            start: self.start,
            end: self.end,
        }
    }
}

impl<S, V> Move<S> for ListReverseMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Undo = ();

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        reverse_is_doable(&self.access(), self.coordinates(), score_director)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        reverse_do_move(&self.access(), self.coordinates(), score_director);
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

    fn telemetry_label(&self) -> &'static str {
        "list_reverse"
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        reverse_tabu_signature(&self.access(), self.coordinates(), score_director)
    }

    fn candidate_trace_identity(&self) -> Option<crate::stats::CandidateTraceIdentity> {
        Some(reverse_candidate_trace_identity(
            &self.access(),
            self.coordinates(),
        ))
    }
}
