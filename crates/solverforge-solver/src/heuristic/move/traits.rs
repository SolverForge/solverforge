// Move trait definition.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::MoveTabuSignature;
use crate::stats::CandidateTraceIdentity;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveAffectedEntity<'a> {
    pub descriptor_index: usize,
    pub entity_index: usize,
    pub variable_name: &'a str,
}

/// A move that modifies one or more planning variables.
///
/// Moves are fully monomorphized for maximum performance - no boxing, no virtual dispatch.
/// Undo is handled by move-owned typed data, not boxed director callbacks.
///
/// # Type Parameters
/// * `S` - The planning solution type
///
/// # Implementation Notes
/// - Moves should be lightweight
/// - Moves are NEVER cloned - ownership transfers via arena indices
/// - Methods are generic over D to allow use with concrete directors
pub trait Move<S: PlanningSolution>: Send + Sync + Debug {
    type Undo: Send;

    /* Returns true if this move can be executed in the current state.

    A move is not doable if:
    - The source value equals the destination value (no change)
    - The move's own structural and value-domain rules reject it
    - The move would violate hard constraints that can be detected early
    */
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool;

    /* Executes this move, modifying the working solution.

    This method modifies the planning variables through the score director.
    Returns concrete undo data that can be passed back to `undo_move`.
    */
    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo;

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo);

    /* Trial path with a retained solve scratch pool of retired undo
    journals. The default ignores the pool and delegates to the plain pair;
    long-journal families override both to recycle undo storage across
    trials, so arbitrary-long edit sets do not grow the heap per trial. */
    fn do_move_with_scratch<D: Director<S>>(
        &self,
        score_director: &mut D,
        journal_pool: &mut Vec<Self::Undo>,
    ) -> Self::Undo {
        let _ = journal_pool;
        self.do_move(score_director)
    }

    fn undo_move_with_scratch<D: Director<S>>(
        &self,
        score_director: &mut D,
        undo: Self::Undo,
        journal_pool: &mut Vec<Self::Undo>,
    ) {
        let _ = journal_pool;
        self.undo_move(score_director, undo)
    }

    /* Doability check that may run an explicit working preview on the LIVE
    director.

    Default: identical to `is_doable` with the director reborrowed shared.
    Sequential/compound families override this where their later legs read
    the earlier legs' effects; the phase trial path calls THIS variant so the
    preview mutation is owned by the phase-level orchestration instead of a
    clone-out substitute director.
    */
    fn is_doable_with_working_preview<D: Director<S>>(&self, score_director: &mut D) -> bool {
        self.is_doable(score_director)
    }

    fn descriptor_index(&self) -> usize;

    fn entity_indices(&self) -> &[usize];

    fn variable_name(&self) -> &str;

    fn telemetry_label(&self) -> &'static str {
        "move"
    }

    fn requires_hard_improvement(&self) -> bool {
        false
    }

    fn requires_score_improvement(&self) -> bool {
        false
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature;

    /// Returns the canonical identity used by opt-in candidate-pull tracing.
    ///
    /// Returns an explicit logical-coordinate identity for candidate tracing.
    ///
    /// This intentionally does *not* derive from tabu metadata: tabu keys may
    /// contain representation-specific hashes. A move family that has not
    /// supplied a cross-representation coordinate grammar returns `None`, and
    /// the resulting trace is explicitly non-qualifying rather than faking a
    /// comparable key.
    fn candidate_trace_identity(&self) -> Option<CandidateTraceIdentity> {
        None
    }

    /// Visits every entity this move changes, passing a concrete visitor by
    /// value. `V` stays monomorphized end to end: no `dyn` dispatch, no
    /// intermediate collection, every affected entity reported exactly once
    /// in the move's own canonical order.
    fn for_each_affected_entity<V: FnMut(MoveAffectedEntity<'_>)>(&self, visitor: &mut V) {
        let descriptor_index = self.descriptor_index();
        let variable_name = self.variable_name();
        for &entity_index in self.entity_indices() {
            visitor(MoveAffectedEntity {
                descriptor_index,
                entity_index,
                variable_name,
            });
        }
    }
}
