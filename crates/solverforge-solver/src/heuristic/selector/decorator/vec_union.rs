/* Vec-backed union move selector for config-driven selector composition.

Unlike `UnionMoveSelector` (which combines exactly two selectors), `VecUnionSelector`
holds a `Vec<Leaf>` of selectors and chains their moves. This is the backbone of
config-driven solver construction where the number of selectors is determined at
runtime from `solver.toml`.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::typed_move_selector::MoveSelector;

/// Combines moves from an arbitrary number of leaf selectors into a single stream.
///
/// Collects all moves from all selectors into a `Vec<M>` and returns an
/// owning iterator. This is intentional: moves are placed in the arena anyway,
/// so the intermediate `Vec` costs no additional allocation beyond what the
/// arena would require.
pub struct VecUnionSelector<S, M, Leaf> {
    selectors: Vec<Leaf>,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, Leaf> VecUnionSelector<S, M, Leaf> {
    pub fn new(selectors: Vec<Leaf>) -> Self {
        Self {
            selectors,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, Leaf: Debug> Debug for VecUnionSelector<S, M, Leaf> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VecUnionSelector")
            .field("selectors", &self.selectors)
            .finish()
    }
}

impl<S, M, Leaf> MoveSelector<S, M> for VecUnionSelector<S, M, Leaf>
where
    S: PlanningSolution,
    M: Move<S>,
    Leaf: MoveSelector<S, M>,
{
    fn iter_moves<'a, D: Director<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = M> + 'a {
        /* Collect all moves upfront. Moves enter the arena immediately after
        iteration, so this Vec is ephemeral and avoids lifetime conflicts
        between multiple borrowed iterators.
        */
        let mut moves: Vec<M> = Vec::new();
        for selector in &self.selectors {
            moves.extend(selector.iter_moves(score_director));
        }
        moves.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.selectors.iter().map(|s| s.size(score_director)).sum()
    }
}
