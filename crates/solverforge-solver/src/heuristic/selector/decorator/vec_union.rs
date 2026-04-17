/* Vec-backed union move selector for config-driven selector composition.

Unlike `UnionMoveSelector` (which combines exactly two selectors), `VecUnionSelector`
holds a `Vec<Leaf>` of selectors and lazily traverses them in order. This is the
backbone of config-driven solver construction where the number of selectors is
determined at runtime from `solver.toml`.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::r#move::MoveArena;
use crate::heuristic::selector::move_selector::MoveSelector;

/// Combines moves from an arbitrary number of leaf selectors into a single stream.
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
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = M> + 'a {
        let cursors: Vec<_> = self
            .selectors
            .iter()
            .map(|selector| selector.open_cursor(score_director))
            .collect();
        cursors.into_iter().flatten()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.selectors.iter().map(|s| s.size(score_director)).sum()
    }

    fn append_moves<D: Director<S>>(&self, score_director: &D, arena: &mut MoveArena<M>) {
        for selector in &self.selectors {
            selector.append_moves(score_director, arena);
        }
    }
}
