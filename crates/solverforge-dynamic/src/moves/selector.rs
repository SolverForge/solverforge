//! Move selector for dynamic solutions — unified change + swap moves.

use solverforge_scoring::ScoreDirector;
use solverforge_solver::heuristic::selector::typed_move_selector::MoveSelector;

use super::swap_move::DynamicSwapMove;
use super::{DynamicEitherMove, DynamicMoveIterator};
use crate::solution::DynamicSolution;

/// Move selector that generates both change moves and swap moves
/// as a unified `DynamicEitherMove` stream.
///
/// Change moves are generated lazily via `DynamicMoveIterator`.
/// Swap moves are generated for triangular entity pairs within each class.
/// Both are chained: all change moves first, then all swap moves.
///
/// The arena in the local search phase populates once and shuffles in-place
/// each step. No per-call allocation or shuffle here.
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

    /// Generate swap moves for all entity pairs within each class.
    fn generate_swap_moves(solution: &DynamicSolution) -> Vec<DynamicEitherMove> {
        let mut swap_count = 0usize;
        for (class_idx, class_def) in solution.descriptor.entity_classes.iter().enumerate() {
            let n = solution
                .entities
                .get(class_idx)
                .map(|e| e.len())
                .unwrap_or(0);
            let vars = class_def.planning_variable_indices.len();
            swap_count += vars * n * n.saturating_sub(1) / 2;
        }

        let mut moves = Vec::with_capacity(swap_count);
        for (class_idx, class_def) in solution.descriptor.entity_classes.iter().enumerate() {
            let entity_count = solution
                .entities
                .get(class_idx)
                .map(|e| e.len())
                .unwrap_or(0);
            if entity_count < 2 {
                continue;
            }

            for &field_idx in &class_def.planning_variable_indices {
                let variable_name = &class_def.fields[field_idx].name;
                // Triangular pairs: (i, j) where i < j
                for i in 0..entity_count {
                    for j in (i + 1)..entity_count {
                        moves.push(DynamicEitherMove::Swap(DynamicSwapMove::new(
                            class_idx,
                            i,
                            j,
                            field_idx,
                            variable_name.clone(),
                        )));
                    }
                }
            }
        }
        moves
    }
}

impl Default for DynamicMoveSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveSelector<DynamicSolution, DynamicEitherMove> for DynamicMoveSelector {
    /// Returns an iterator over all change and swap moves.
    ///
    /// Moves are yielded in deterministic order. The local search phase's
    /// arena handles shuffling in-place each step — no allocation here.
    fn iter_moves<'a, D: ScoreDirector<DynamicSolution>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = DynamicEitherMove> + 'a {
        let solution = score_director.working_solution();

        let change_iter = DynamicMoveIterator::new(solution).map(DynamicEitherMove::Change);
        let swap_moves = Self::generate_swap_moves(solution);

        change_iter.chain(swap_moves)
    }

    fn size<D: ScoreDirector<DynamicSolution>>(&self, score_director: &D) -> usize {
        let solution = score_director.working_solution();
        let change_count = DynamicMoveIterator::new(solution).count();

        let mut swap_count = 0usize;
        for (class_idx, class_def) in solution.descriptor.entity_classes.iter().enumerate() {
            let n = solution
                .entities
                .get(class_idx)
                .map(|e| e.len())
                .unwrap_or(0);
            let vars = class_def.planning_variable_indices.len();
            swap_count += vars * n * n.saturating_sub(1) / 2;
        }

        change_count + swap_count
    }
}
