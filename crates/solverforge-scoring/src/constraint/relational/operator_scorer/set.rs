//! The scorer-set contract shared terminals score through.
//!
//! Weighted counterparts take one multiplier per scorer, in chain order. Each
//! scorer scales its own contribution, so terminals sharing one operator keep
//! independent multipliers while the operator state stays shared.

use solverforge_core::{score::Score, ConstraintRef};

use crate::api::{
    analysis::ConstraintAnalysis,
    constraint_set::{ConstraintMetadata, ConstraintResult},
};
use crate::stream::relational::operator::Operator;
use crate::stream::relational::RowHandle;

/// The named terminals scoring one shared operator.
///
/// A shared terminal advances its operator once per notification and hands
/// the removed and inserted row handles to its scorer set, which re-scores
/// exactly those rows for every terminal. `()` is the empty set,
/// [`OperatorScorer`](super::OperatorScorer) scores one terminal, and
/// [`ScorerChain`](super::ScorerChain) composes two sets in authored order.
/// The methods mirror [`ConstraintSet`](crate::api::constraint_set::ConstraintSet),
/// taking the shared operator as an argument.
pub trait OperatorScorerSet<S: 'static, O: Operator<S>, Sc: Score>: Send + Sync {
    /// Sums every terminal over the operator's current rows, without retained state.
    fn evaluate(&self, operator: &O, solution: &S) -> Sc;
    /// Scores every current row, retains each row's score, and returns the sum.
    fn initialize(&mut self, operator: &O, solution: &S) -> Sc;
    /// Retracts the retained scores of `removed`, scores `inserted`, and returns
    /// the summed delta with an overflow flag.
    fn apply_changes(
        &mut self,
        operator: &O,
        solution: &S,
        removed: &[RowHandle],
        inserted: &[RowHandle],
    ) -> (Sc, bool);
    /// Number of named terminals in the set.
    fn constraint_count(&self) -> usize;
    /// One metadata entry per terminal, in authored order.
    fn metadata(&self) -> Vec<ConstraintMetadata<'_>>;
    /// Each terminal's total, in authored order.
    fn evaluate_each<'a>(&'a self, operator: &O, solution: &S) -> Vec<ConstraintResult<'a, Sc>>;
    /// Each terminal's total with its matches, in authored order.
    fn evaluate_detailed<'a>(
        &'a self,
        operator: &O,
        solution: &S,
    ) -> Vec<ConstraintAnalysis<'a, Sc>>;
    /// Drops every retained row score.
    fn reset(&mut self);
    /// The retained signed score of row `handle` for a single-terminal set;
    /// `None` when the set does not score rows on its own.
    fn score_for(&self, _handle: RowHandle) -> Option<Sc> {
        None
    }

    /// Appends each terminal's constraint reference, one weight slot each.
    fn weight_refs<'a>(&'a self, refs: &mut Vec<&'a ConstraintRef>);
    /// [`evaluate`](Self::evaluate) with each terminal scaled by its multiplier.
    fn evaluate_weighted(&self, operator: &O, solution: &S, multipliers: &[u32]) -> Sc;
    /// [`initialize`](Self::initialize) with each terminal scaled by its multiplier.
    fn initialize_weighted(&mut self, operator: &O, solution: &S, multipliers: &[u32]) -> Sc;
    /// [`apply_changes`](Self::apply_changes) with each terminal scaled by its
    /// multiplier.
    fn apply_changes_weighted(
        &mut self,
        operator: &O,
        solution: &S,
        removed: &[RowHandle],
        inserted: &[RowHandle],
        multipliers: &[u32],
    ) -> (Sc, bool);
    /// [`evaluate_each`](Self::evaluate_each) with each terminal scaled by its
    /// multiplier.
    fn evaluate_each_weighted<'a>(
        &'a self,
        operator: &O,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintResult<'a, Sc>>;
    /// [`evaluate_detailed`](Self::evaluate_detailed) with each terminal scaled
    /// by its multiplier.
    fn evaluate_detailed_weighted<'a>(
        &'a self,
        operator: &O,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintAnalysis<'a, Sc>>;
}
