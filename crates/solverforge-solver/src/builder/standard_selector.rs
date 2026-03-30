// Standard variable move selector enum and builder.

use std::fmt::Debug;

use solverforge_config::MoveSelectorConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::EitherMove;
use crate::heuristic::selector::decorator::VecUnionSelector;
use crate::heuristic::selector::typed_move_selector::MoveSelector;
use crate::heuristic::selector::{
    EitherChangeMoveSelector, EitherSwapMoveSelector, FromSolutionEntitySelector,
};

use super::context::StandardContext;

/// A monomorphized leaf selector for basic (non-list) planning variables.
///
/// Wraps either a change or swap move selector in a uniform enum so that
/// `VecUnionSelector<S, EitherMove<S, usize>, StandardLeafSelector<S>>` has a
/// single concrete type regardless of which selectors are active.
pub enum StandardLeafSelector<S: PlanningSolution> {
    // A change move selector yielding `EitherMove::Change`.
    Change(
        EitherChangeMoveSelector<
            S,
            usize,
            FromSolutionEntitySelector,
            crate::heuristic::selector::typed_value::StaticTypedValueSelector<S, usize>,
        >,
    ),
    // A swap move selector yielding `EitherMove::Swap`.
    Swap(EitherSwapMoveSelector<S, usize, FromSolutionEntitySelector, FromSolutionEntitySelector>),
}

impl<S: PlanningSolution> Debug for StandardLeafSelector<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Change(s) => write!(f, "StandardLeafSelector::Change({s:?})"),
            Self::Swap(s) => write!(f, "StandardLeafSelector::Swap({s:?})"),
        }
    }
}

impl<S> MoveSelector<S, EitherMove<S, usize>> for StandardLeafSelector<S>
where
    S: PlanningSolution,
{
    fn iter_moves<'a, D: Director<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = EitherMove<S, usize>> + 'a {
        match self {
            Self::Change(s) => {
                let moves: Vec<_> = s.iter_moves(score_director).collect();
                moves.into_iter()
            }
            Self::Swap(s) => {
                let moves: Vec<_> = s.iter_moves(score_director).collect();
                moves.into_iter()
            }
        }
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Change(s) => s.size(score_director),
            Self::Swap(s) => s.size(score_director),
        }
    }
}

/// Builder that constructs a `VecUnionSelector` of `StandardLeafSelector` from config.
pub struct StandardMoveSelectorBuilder;

impl StandardMoveSelectorBuilder {
    /// Builds a `VecUnionSelector` from the given move selector config and domain context.
    ///
    /// - `ChangeMoveSelector` → `StandardLeafSelector::Change`
    /// - `SwapMoveSelector` → `StandardLeafSelector::Swap`
    /// - `UnionMoveSelector` → flattens children recursively
    /// - `None` → default: Change + Swap
    pub fn build<S>(
        config: Option<&MoveSelectorConfig>,
        ctx: &StandardContext<S>,
    ) -> VecUnionSelector<S, EitherMove<S, usize>, StandardLeafSelector<S>>
    where
        S: PlanningSolution,
    {
        let mut leaves: Vec<StandardLeafSelector<S>> = Vec::new();
        match config {
            None => {
                Self::push_change(&mut leaves, ctx);
                Self::push_swap(&mut leaves, ctx);
            }
            Some(cfg) => Self::collect_leaves(cfg, ctx, &mut leaves),
        }
        VecUnionSelector::new(leaves)
    }

    fn collect_leaves<S>(
        config: &MoveSelectorConfig,
        ctx: &StandardContext<S>,
        out: &mut Vec<StandardLeafSelector<S>>,
    ) where
        S: PlanningSolution,
    {
        match config {
            MoveSelectorConfig::ChangeMoveSelector(_) => Self::push_change(out, ctx),
            MoveSelectorConfig::SwapMoveSelector(_) => Self::push_swap(out, ctx),
            MoveSelectorConfig::UnionMoveSelector(u) => {
                for child in &u.selectors {
                    Self::collect_leaves(child, ctx, out);
                }
            }
            // All other variants are list selectors — ignore for basic solver
            _ => {
                // Default to change + swap if unknown selector type is specified
                Self::push_change(out, ctx);
                Self::push_swap(out, ctx);
            }
        }
    }

    fn push_change<S>(out: &mut Vec<StandardLeafSelector<S>>, ctx: &StandardContext<S>)
    where
        S: PlanningSolution,
    {
        out.push(StandardLeafSelector::Change(
            EitherChangeMoveSelector::simple(
                ctx.get_variable,
                ctx.set_variable,
                ctx.descriptor_index,
                ctx.variable_field,
                ctx.values.clone(),
            ),
        ));
    }

    fn push_swap<S>(out: &mut Vec<StandardLeafSelector<S>>, ctx: &StandardContext<S>)
    where
        S: PlanningSolution,
    {
        out.push(StandardLeafSelector::Swap(EitherSwapMoveSelector::simple(
            ctx.get_variable,
            ctx.set_variable,
            ctx.descriptor_index,
            ctx.variable_field,
        )));
    }
}
