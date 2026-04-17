use std::fmt::{self, Debug};
use std::ops::Range;

use solverforge_config::MoveSelectorConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{EitherMove, MoveArena};
use crate::heuristic::selector::decorator::VecUnionSelector;
use crate::heuristic::selector::{
    ChangeMoveSelector, FromSolutionEntitySelector, MoveSelector, PerEntitySliceValueSelector,
    RangeValueSelector, SwapMoveSelector, ValueSelector,
};

use super::{StandardContext, StandardValueSource, StandardVariableContext};

pub type StandardSelector<S> = VecUnionSelector<S, EitherMove<S, usize>, StandardLeafSelector<S>>;

pub enum StandardValueSelector<S> {
    Empty,
    CountableRange { from: usize, to: usize },
    SolutionCount(RangeValueSelector<S>),
    EntitySlice(PerEntitySliceValueSelector<S, usize>),
}

impl<S> StandardValueSelector<S> {
    fn from_source(source: StandardValueSource<S>) -> Self {
        match source {
            StandardValueSource::Empty => Self::Empty,
            StandardValueSource::CountableRange { from, to } => Self::CountableRange { from, to },
            StandardValueSource::SolutionCount { count_fn } => {
                Self::SolutionCount(RangeValueSelector::new(count_fn))
            }
            StandardValueSource::EntitySlice { values_for_entity } => {
                Self::EntitySlice(PerEntitySliceValueSelector::new(values_for_entity))
            }
        }
    }
}

impl<S> Debug for StandardValueSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "StandardValueSelector::Empty"),
            Self::CountableRange { from, to } => {
                write!(f, "StandardValueSelector::CountableRange({from}..{to})")
            }
            Self::SolutionCount(_) => write!(f, "StandardValueSelector::SolutionCount(..)"),
            Self::EntitySlice(_) => write!(f, "StandardValueSelector::EntitySlice(..)"),
        }
    }
}

impl<S> ValueSelector<S, usize> for StandardValueSelector<S>
where
    S: PlanningSolution,
{
    fn iter_typed<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        descriptor_index: usize,
        entity_index: usize,
    ) -> impl Iterator<Item = usize> + 'a {
        match self {
            Self::Empty => StandardValueIter::Empty,
            Self::CountableRange { from, to } => StandardValueIter::CountableRange(*from..*to),
            Self::SolutionCount(selector) => StandardValueIter::SolutionCount(selector.iter_typed(
                score_director,
                descriptor_index,
                entity_index,
            )),
            Self::EntitySlice(selector) => StandardValueIter::EntitySlice(selector.iter_typed(
                score_director,
                descriptor_index,
                entity_index,
            )),
        }
    }

    fn size<D: Director<S>>(
        &self,
        score_director: &D,
        descriptor_index: usize,
        entity_index: usize,
    ) -> usize {
        match self {
            Self::Empty => 0,
            Self::CountableRange { from, to } => to.saturating_sub(*from),
            Self::SolutionCount(selector) => {
                selector.size(score_director, descriptor_index, entity_index)
            }
            Self::EntitySlice(selector) => {
                selector.size(score_director, descriptor_index, entity_index)
            }
        }
    }
}

enum StandardValueIter<A, B> {
    Empty,
    CountableRange(Range<usize>),
    SolutionCount(A),
    EntitySlice(B),
}

impl<A, B> Iterator for StandardValueIter<A, B>
where
    A: Iterator<Item = usize>,
    B: Iterator<Item = usize>,
{
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty => None,
            Self::CountableRange(iter) => iter.next(),
            Self::SolutionCount(iter) => iter.next(),
            Self::EntitySlice(iter) => iter.next(),
        }
    }
}

type StandardChangeSelector<S> =
    ChangeMoveSelector<S, usize, FromSolutionEntitySelector, StandardValueSelector<S>>;
type StandardSwapSelector<S> =
    SwapMoveSelector<S, usize, FromSolutionEntitySelector, FromSolutionEntitySelector>;

pub enum StandardLeafSelector<S> {
    Change(StandardChangeSelector<S>),
    Swap(StandardSwapSelector<S>),
}

impl<S> Debug for StandardLeafSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Change(selector) => selector.fmt(f),
            Self::Swap(selector) => selector.fmt(f),
        }
    }
}

impl<S> MoveSelector<S, EitherMove<S, usize>> for StandardLeafSelector<S>
where
    S: PlanningSolution,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = EitherMove<S, usize>> + 'a {
        match self {
            Self::Change(selector) => StandardLeafIter::Change(
                selector.open_cursor(score_director).map(EitherMove::Change),
            ),
            Self::Swap(selector) => {
                StandardLeafIter::Swap(selector.open_cursor(score_director).map(EitherMove::Swap))
            }
        }
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Change(selector) => selector.size(score_director),
            Self::Swap(selector) => selector.size(score_director),
        }
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<EitherMove<S, usize>>,
    ) {
        match self {
            Self::Change(selector) => {
                arena.extend(selector.open_cursor(score_director).map(EitherMove::Change))
            }
            Self::Swap(selector) => {
                arena.extend(selector.open_cursor(score_director).map(EitherMove::Swap))
            }
        }
    }
}

enum StandardLeafIter<A, B> {
    Change(A),
    Swap(B),
}

impl<T, A, B> Iterator for StandardLeafIter<A, B>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Change(iter) => iter.next(),
            Self::Swap(iter) => iter.next(),
        }
    }
}

pub fn build_standard_move_selector<S>(
    config: Option<&MoveSelectorConfig>,
    standard_ctx: &StandardContext<S>,
) -> StandardSelector<S>
where
    S: PlanningSolution + 'static,
{
    let mut leaves = Vec::new();
    collect_standard_leaf_selectors(config, standard_ctx, &mut leaves);
    assert!(
        !leaves.is_empty(),
        "stock move selector configuration produced no standard neighborhoods"
    );
    VecUnionSelector::new(leaves)
}

fn collect_standard_leaf_selectors<S>(
    config: Option<&MoveSelectorConfig>,
    standard_ctx: &StandardContext<S>,
    leaves: &mut Vec<StandardLeafSelector<S>>,
) where
    S: PlanningSolution + 'static,
{
    fn push_change<S: PlanningSolution + 'static>(
        ctx: &StandardVariableContext<S>,
        leaves: &mut Vec<StandardLeafSelector<S>>,
    ) {
        leaves.push(StandardLeafSelector::Change(ChangeMoveSelector::new(
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            StandardValueSelector::from_source(ctx.value_source),
            ctx.getter,
            ctx.setter,
            ctx.descriptor_index,
            ctx.variable_name,
        )));
    }

    fn push_swap<S: PlanningSolution + 'static>(
        ctx: &StandardVariableContext<S>,
        leaves: &mut Vec<StandardLeafSelector<S>>,
    ) {
        leaves.push(StandardLeafSelector::Swap(SwapMoveSelector::new(
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            ctx.getter,
            ctx.setter,
            ctx.descriptor_index,
            ctx.variable_name,
        )));
    }

    fn collect<S: PlanningSolution + 'static>(
        cfg: &MoveSelectorConfig,
        standard_ctx: &StandardContext<S>,
        leaves: &mut Vec<StandardLeafSelector<S>>,
    ) {
        match cfg {
            MoveSelectorConfig::ChangeMoveSelector(change) => {
                let matched: Vec<_> = standard_ctx
                    .variables()
                    .iter()
                    .cloned()
                    .filter(|ctx| {
                        ctx.matches_target(
                            change.target.entity_class.as_deref(),
                            change.target.variable_name.as_deref(),
                        )
                    })
                    .collect();
                assert!(
                    !matched.is_empty(),
                    "change_move selector matched no standard planning variables for entity_class={:?} variable_name={:?}",
                    change.target.entity_class,
                    change.target.variable_name
                );
                for ctx in matched {
                    push_change(&ctx, leaves);
                }
            }
            MoveSelectorConfig::SwapMoveSelector(swap) => {
                let matched: Vec<_> = standard_ctx
                    .variables()
                    .iter()
                    .cloned()
                    .filter(|ctx| {
                        ctx.matches_target(
                            swap.target.entity_class.as_deref(),
                            swap.target.variable_name.as_deref(),
                        )
                    })
                    .collect();
                assert!(
                    !matched.is_empty(),
                    "swap_move selector matched no standard planning variables for entity_class={:?} variable_name={:?}",
                    swap.target.entity_class,
                    swap.target.variable_name
                );
                for ctx in matched {
                    push_swap(&ctx, leaves);
                }
            }
            MoveSelectorConfig::UnionMoveSelector(union) => {
                for child in &union.selectors {
                    collect(child, standard_ctx, leaves);
                }
            }
            MoveSelectorConfig::SelectedCountLimitMoveSelector(_) => {
                panic!(
                    "selected_count_limit_move_selector must be handled by the unified stock runtime"
                );
            }
            MoveSelectorConfig::ListChangeMoveSelector(_)
            | MoveSelectorConfig::NearbyListChangeMoveSelector(_)
            | MoveSelectorConfig::ListSwapMoveSelector(_)
            | MoveSelectorConfig::NearbyListSwapMoveSelector(_)
            | MoveSelectorConfig::SubListChangeMoveSelector(_)
            | MoveSelectorConfig::SubListSwapMoveSelector(_)
            | MoveSelectorConfig::ListReverseMoveSelector(_)
            | MoveSelectorConfig::KOptMoveSelector(_)
            | MoveSelectorConfig::ListRuinMoveSelector(_) => {
                panic!("list move selector configured against a standard-variable stock context");
            }
            MoveSelectorConfig::CartesianProductMoveSelector(_) => {
                panic!("cartesian_product move selectors are not supported in stock solving");
            }
        }
    }

    match config {
        Some(cfg) => collect(cfg, standard_ctx, leaves),
        None => {
            for ctx in standard_ctx.variables() {
                push_change(ctx, leaves);
                push_swap(ctx, leaves);
            }
        }
    }
}

#[cfg(test)]
#[path = "standard_selector_tests.rs"]
mod tests;
