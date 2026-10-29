/* Score bounder for exhaustive search pruning.

Bounders calculate optimistic and pessimistic score bounds
that enable branch-and-bound pruning.
*/

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

/// Calculates score bounds for exhaustive search pruning.
///
/// The bounder estimates the best possible score that can be achieved
/// from a partial solution state. If this optimistic bound is worse than
/// the best complete solution found so far, the branch can be pruned.
pub trait ScoreBounder<S: PlanningSolution, D: Director<S>>: Send + Debug {
    /* Calculates the optimistic bound for the current solution state.

    The optimistic bound is an upper bound on the score achievable
    from this partial solution. It should be:
    - Fast to compute
    - Greater than or equal to any actual achievable score

    Returns `None` if no bound can be computed.
    */
    fn calculate_optimistic_bound(&self, score_director: &D) -> Option<S::Score>;

    /* Calculates the pessimistic bound for the current solution state.

    The pessimistic bound is a lower bound on the score achievable
    from this partial solution. This is less commonly used but can
    help with certain heuristics.

    Returns `None` if no bound can be computed.
    */
    fn calculate_pessimistic_bound(&self, score_director: &D) -> Option<S::Score> {
        // Default: no pessimistic bound
        let _ = score_director;
        None
    }
}
