//! Move selector for dynamic solutions.

use solverforge_scoring::ScoreDirector;
use solverforge_solver::heuristic::selector::typed_move_selector::MoveSelector;

use super::{DynamicChangeMove, DynamicMoveIterator};
use crate::solution::DynamicSolution;

#[derive(Debug)]
pub struct DynamicMoveSelector {
    _phantom: std::marker::PhantomData<()>,
}

impl DynamicMoveSelector {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Returns a lazy iterator over all possible change moves for the solution.
    ///
    /// This method generates moves on-demand using `DynamicMoveIterator`, which
    /// significantly reduces memory usage for large solutions compared to
    /// pre-computing all moves into a `Vec`.
    ///
    /// The moves are generated in deterministic order (by class, entity, variable, value).
    /// If randomized order is needed (e.g., for `FirstAcceptedForager`), the caller
    /// should collect and shuffle the moves, or use `generate_moves_shuffled`.
    pub fn generate_moves<'a>(&self, solution: &'a DynamicSolution) -> DynamicMoveIterator<'a> {
        DynamicMoveIterator::new(solution)
    }

    /// Returns all possible change moves as a shuffled Vec.
    ///
    /// This method collects all moves and shuffles them for randomized selection,
    /// which is important for foragers like `FirstAcceptedForager`.
    ///
    /// For memory-constrained scenarios with many moves, prefer using
    /// `generate_moves()` to get a lazy iterator.
    pub fn generate_moves_shuffled(&self, solution: &DynamicSolution) -> Vec<DynamicChangeMove> {
        use rand::seq::SliceRandom;

        let mut moves: Vec<_> = self.generate_moves(solution).collect();

        // Shuffle moves for randomized selection (important for FirstAcceptedForager)
        let mut rng = rand::rng();
        moves.shuffle(&mut rng);

        moves
    }
}

impl Default for DynamicMoveSelector {
    fn default() -> Self {
        Self::new()
    }
}

// Implement the real MoveSelector trait
impl MoveSelector<DynamicSolution, DynamicChangeMove> for DynamicMoveSelector {
    /// Returns an iterator over all possible change moves in randomized order.
    ///
    /// This implementation leverages `DynamicMoveIterator` for the initial move generation,
    /// then collects and shuffles the moves for randomized selection. The shuffling is
    /// critical for effective local search with `FirstAcceptedForager`, which relies on
    /// randomized move order to escape local optima and explore the solution space.
    ///
    /// **Design decision**: While lazy iteration without shuffling would save memory,
    /// it causes the solver to get stuck in local optima because moves are always
    /// evaluated in the same deterministic order. The shuffled approach ensures:
    /// - Effective exploration with `FirstAcceptedForager`
    /// - Consistent solver performance across different problem instances
    /// - Better solution quality within time limits
    ///
    /// For memory-constrained scenarios where deterministic order is acceptable,
    /// use `generate_moves()` directly to get a lazy iterator.
    fn iter_moves<'a, D: ScoreDirector<DynamicSolution>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = DynamicChangeMove> + 'a> {
        let solution = score_director.working_solution();
        // Collect moves from lazy iterator and shuffle for randomized selection
        // This is necessary for FirstAcceptedForager to work effectively
        let moves = self.generate_moves_shuffled(solution);
        Box::new(moves.into_iter())
    }

    fn size<D: ScoreDirector<DynamicSolution>>(&self, score_director: &D) -> usize {
        // Use lazy iterator to count without allocating all moves
        self.generate_moves(score_director.working_solution())
            .count()
    }
}
