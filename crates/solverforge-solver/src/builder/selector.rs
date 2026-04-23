use std::fmt::{self, Debug};

use solverforge_config::{
    AcceptorConfig, ChangeMoveConfig, ListReverseMoveConfig, LocalSearchConfig, MoveSelectorConfig,
    NearbyListChangeMoveConfig, NearbyListSwapMoveConfig, VariableTargetConfig, VndConfig,
};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};

use crate::heuristic::r#move::{
    ListMoveUnion, Move, MoveArena, MoveTabuSignature, ScalarMoveUnion, SequentialCompositeMove,
};
use crate::heuristic::selector::decorator::{
    CartesianProductCursor, CartesianProductSelector, VecUnionSelector,
};
use crate::heuristic::selector::move_selector::{
    ArenaMoveCursor, MoveCandidateRef, MoveCursor, MoveSelector,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::dynamic_vnd::DynamicVndPhase;
use crate::phase::localsearch::{
    AcceptedCountForager, LocalSearchPhase, SimulatedAnnealingAcceptor,
};

use super::acceptor::{AcceptorBuilder, AnyAcceptor};
use super::context::ModelContext;
use super::forager::{AnyForager, ForagerBuilder};
use super::list_selector::{ListLeafSelector, ListMoveSelectorBuilder};
use super::scalar_selector::{build_scalar_flat_selector, ScalarLeafSelector};

type LeafSelector<S, V, DM, IDM> =
    VecUnionSelector<S, NeighborhoodMove<S, V>, NeighborhoodLeaf<S, V, DM, IDM>>;

pub enum NeighborhoodMove<S, V> {
    Scalar(ScalarMoveUnion<S, usize>),
    List(ListMoveUnion<S, V>),
    Composite(SequentialCompositeMove<S, NeighborhoodMove<S, V>>),
}

impl<S, V> Clone for NeighborhoodMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn clone(&self) -> Self {
        match self {
            Self::Scalar(m) => Self::Scalar(m.clone()),
            Self::List(m) => Self::List(m.clone()),
            Self::Composite(m) => Self::Composite(m.clone()),
        }
    }
}

impl<S, V> Debug for NeighborhoodMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(m) => write!(f, "NeighborhoodMove::Scalar({m:?})"),
            Self::List(m) => write!(f, "NeighborhoodMove::List({m:?})"),
            Self::Composite(m) => write!(f, "NeighborhoodMove::Composite({m:?})"),
        }
    }
}

impl<S, V> Move<S> for NeighborhoodMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::Scalar(m) => m.is_doable(score_director),
            Self::List(m) => m.is_doable(score_director),
            Self::Composite(m) => m.is_doable(score_director),
        }
    }

    fn do_move<D: solverforge_scoring::Director<S>>(&self, score_director: &mut D) {
        match self {
            Self::Scalar(m) => m.do_move(score_director),
            Self::List(m) => m.do_move(score_director),
            Self::Composite(m) => m.do_move(score_director),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Scalar(m) => m.descriptor_index(),
            Self::List(m) => m.descriptor_index(),
            Self::Composite(m) => m.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Scalar(m) => m.entity_indices(),
            Self::List(m) => m.entity_indices(),
            Self::Composite(m) => m.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Scalar(m) => m.variable_name(),
            Self::List(m) => m.variable_name(),
            Self::Composite(m) => m.variable_name(),
        }
    }

    fn tabu_signature<D: solverforge_scoring::Director<S>>(
        &self,
        score_director: &D,
    ) -> MoveTabuSignature {
        match self {
            Self::Scalar(m) => m.tabu_signature(score_director),
            Self::List(m) => m.tabu_signature(score_director),
            Self::Composite(m) => m.tabu_signature(score_director),
        }
    }
}

pub enum NeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    Scalar(ScalarLeafSelector<S>),
    List(ListLeafSelector<S, V, DM, IDM>),
}

impl<S, V, DM, IDM> Debug for NeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(selector) => write!(f, "NeighborhoodLeaf::Scalar({selector:?})"),
            Self::List(selector) => write!(f, "NeighborhoodLeaf::List({selector:?})"),
        }
    }
}

impl<S, V, DM, IDM> MoveSelector<S, NeighborhoodMove<S, V>> for NeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    type Cursor<'a>
        = ArenaMoveCursor<S, NeighborhoodMove<S, V>>
    where
        Self: 'a;

    fn open_cursor<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
    ) -> Self::Cursor<'a> {
        match self {
            Self::Scalar(selector) => ArenaMoveCursor::from_moves(
                selector
                    .iter_moves(score_director)
                    .map(NeighborhoodMove::Scalar),
            ),
            Self::List(selector) => ArenaMoveCursor::from_moves(
                selector
                    .iter_moves(score_director)
                    .map(NeighborhoodMove::List),
            ),
        }
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Scalar(selector) => selector.size(score_director),
            Self::List(selector) => selector.size(score_director),
        }
    }

    fn append_moves<D: solverforge_scoring::Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<NeighborhoodMove<S, V>>,
    ) {
        arena.extend(self.open_cursor(score_director));
    }
}

pub enum Neighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    Flat(LeafSelector<S, V, DM, IDM>),
    Limited {
        selector: LeafSelector<S, V, DM, IDM>,
        selected_count_limit: usize,
    },
    Cartesian(CartesianNeighborhoodSelector<S, V, DM, IDM>),
}

pub enum CartesianChildSelector<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    Flat(LeafSelector<S, V, DM, IDM>),
    Limited {
        selector: LeafSelector<S, V, DM, IDM>,
        selected_count_limit: usize,
    },
}

type CartesianNeighborhoodSelector<S, V, DM, IDM> = CartesianProductSelector<
    S,
    NeighborhoodMove<S, V>,
    CartesianChildSelector<S, V, DM, IDM>,
    CartesianChildSelector<S, V, DM, IDM>,
>;

pub enum CartesianChildCursor<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    Flat(ArenaMoveCursor<S, NeighborhoodMove<S, V>>),
    Limited(ArenaMoveCursor<S, NeighborhoodMove<S, V>>),
}

impl<S, V> MoveCursor<S, NeighborhoodMove<S, V>> for CartesianChildCursor<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(
        &mut self,
    ) -> Option<(usize, MoveCandidateRef<'_, S, NeighborhoodMove<S, V>>)> {
        match self {
            Self::Flat(cursor) => cursor.next_candidate(),
            Self::Limited(cursor) => cursor.next_candidate(),
        }
    }

    fn candidate(&self, index: usize) -> Option<MoveCandidateRef<'_, S, NeighborhoodMove<S, V>>> {
        match self {
            Self::Flat(cursor) => cursor.candidate(index),
            Self::Limited(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: usize) -> NeighborhoodMove<S, V> {
        match self {
            Self::Flat(cursor) => cursor.take_candidate(index),
            Self::Limited(cursor) => cursor.take_candidate(index),
        }
    }
}

pub enum NeighborhoodCursor<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    Flat(ArenaMoveCursor<S, NeighborhoodMove<S, V>>),
    Limited(ArenaMoveCursor<S, NeighborhoodMove<S, V>>),
    Cartesian(CartesianProductCursor<S, NeighborhoodMove<S, V>>),
}

impl<S, V> MoveCursor<S, NeighborhoodMove<S, V>> for NeighborhoodCursor<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(
        &mut self,
    ) -> Option<(usize, MoveCandidateRef<'_, S, NeighborhoodMove<S, V>>)> {
        match self {
            Self::Flat(cursor) => cursor.next_candidate(),
            Self::Limited(cursor) => cursor.next_candidate(),
            Self::Cartesian(cursor) => cursor.next_candidate(),
        }
    }

    fn candidate(&self, index: usize) -> Option<MoveCandidateRef<'_, S, NeighborhoodMove<S, V>>> {
        match self {
            Self::Flat(cursor) => cursor.candidate(index),
            Self::Limited(cursor) => cursor.candidate(index),
            Self::Cartesian(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: usize) -> NeighborhoodMove<S, V> {
        match self {
            Self::Flat(cursor) => cursor.take_candidate(index),
            Self::Limited(cursor) => cursor.take_candidate(index),
            Self::Cartesian(cursor) => cursor.take_candidate(index),
        }
    }
}

impl<S, V, DM, IDM> Debug for Neighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flat(selector) => write!(f, "Neighborhood::Flat({selector:?})"),
            Self::Limited {
                selector,
                selected_count_limit,
            } => f
                .debug_struct("Neighborhood::Limited")
                .field("selector", selector)
                .field("selected_count_limit", selected_count_limit)
                .finish(),
            Self::Cartesian(selector) => write!(f, "Neighborhood::Cartesian({selector:?})"),
        }
    }
}

impl<S, V, DM, IDM> MoveSelector<S, NeighborhoodMove<S, V>> for Neighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    type Cursor<'a>
        = NeighborhoodCursor<S, V>
    where
        Self: 'a;

    fn open_cursor<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
    ) -> Self::Cursor<'a> {
        match self {
            Self::Flat(selector) => NeighborhoodCursor::Flat(ArenaMoveCursor::from_moves(
                selector.iter_moves(score_director),
            )),
            Self::Limited {
                selector,
                selected_count_limit,
            } => NeighborhoodCursor::Limited(ArenaMoveCursor::from_moves(
                selector
                    .iter_moves(score_director)
                    .take(*selected_count_limit),
            )),
            Self::Cartesian(selector) => {
                NeighborhoodCursor::Cartesian(selector.open_cursor(score_director))
            }
        }
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Flat(selector) => selector.size(score_director),
            Self::Limited {
                selector,
                selected_count_limit,
            } => selector.size(score_director).min(*selected_count_limit),
            Self::Cartesian(selector) => selector.size(score_director),
        }
    }
}

impl<S, V, DM, IDM> Debug for CartesianChildSelector<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flat(selector) => write!(f, "CartesianChildSelector::Flat({selector:?})"),
            Self::Limited {
                selector,
                selected_count_limit,
            } => f
                .debug_struct("CartesianChildSelector::Limited")
                .field("selector", selector)
                .field("selected_count_limit", selected_count_limit)
                .finish(),
        }
    }
}

impl<S, V, DM, IDM> MoveSelector<S, NeighborhoodMove<S, V>>
    for CartesianChildSelector<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    type Cursor<'a>
        = CartesianChildCursor<S, V>
    where
        Self: 'a;

    fn open_cursor<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
    ) -> Self::Cursor<'a> {
        match self {
            Self::Flat(selector) => CartesianChildCursor::Flat(ArenaMoveCursor::from_moves(
                selector.iter_moves(score_director),
            )),
            Self::Limited {
                selector,
                selected_count_limit,
            } => CartesianChildCursor::Limited(ArenaMoveCursor::from_moves(
                selector
                    .iter_moves(score_director)
                    .take(*selected_count_limit),
            )),
        }
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Flat(selector) => selector.size(score_director),
            Self::Limited {
                selector,
                selected_count_limit,
            } => selector.size(score_director).min(*selected_count_limit),
        }
    }
}

pub type Selector<S, V, DM, IDM> =
    VecUnionSelector<S, NeighborhoodMove<S, V>, Neighborhood<S, V, DM, IDM>>;

pub type LocalSearch<S, V, DM, IDM> = LocalSearchPhase<
    S,
    NeighborhoodMove<S, V>,
    Selector<S, V, DM, IDM>,
    AnyAcceptor<S>,
    AnyForager<S>,
>;

pub type Vnd<S, V, DM, IDM> =
    DynamicVndPhase<S, NeighborhoodMove<S, V>, Neighborhood<S, V, DM, IDM>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectorFamily {
    Scalar,
    List,
    Mixed,
    Unsupported,
}

fn selector_family(config: &MoveSelectorConfig) -> SelectorFamily {
    match config {
        MoveSelectorConfig::ChangeMoveSelector(_)
        | MoveSelectorConfig::SwapMoveSelector(_)
        | MoveSelectorConfig::NearbyChangeMoveSelector(_)
        | MoveSelectorConfig::NearbySwapMoveSelector(_)
        | MoveSelectorConfig::PillarChangeMoveSelector(_)
        | MoveSelectorConfig::PillarSwapMoveSelector(_)
        | MoveSelectorConfig::RuinRecreateMoveSelector(_) => SelectorFamily::Scalar,
        MoveSelectorConfig::ListChangeMoveSelector(_)
        | MoveSelectorConfig::NearbyListChangeMoveSelector(_)
        | MoveSelectorConfig::ListSwapMoveSelector(_)
        | MoveSelectorConfig::NearbyListSwapMoveSelector(_)
        | MoveSelectorConfig::SublistChangeMoveSelector(_)
        | MoveSelectorConfig::SublistSwapMoveSelector(_)
        | MoveSelectorConfig::ListReverseMoveSelector(_)
        | MoveSelectorConfig::KOptMoveSelector(_)
        | MoveSelectorConfig::ListRuinMoveSelector(_) => SelectorFamily::List,
        MoveSelectorConfig::LimitedNeighborhood(limit) => selector_family(limit.selector.as_ref()),
        MoveSelectorConfig::UnionMoveSelector(union) => {
            let mut family = None;
            for child in &union.selectors {
                let child_family = selector_family(child);
                if child_family == SelectorFamily::Unsupported {
                    return SelectorFamily::Unsupported;
                }
                family = Some(match family {
                    None => child_family,
                    Some(current) if current == child_family => current,
                    Some(_) => SelectorFamily::Mixed,
                });
                if family == Some(SelectorFamily::Mixed) {
                    return SelectorFamily::Mixed;
                }
            }
            family.unwrap_or(SelectorFamily::Mixed)
        }
        MoveSelectorConfig::CartesianProductMoveSelector(_) => SelectorFamily::Unsupported,
    }
}

fn selector_requires_score_during_move(config: &MoveSelectorConfig) -> bool {
    match config {
        MoveSelectorConfig::RuinRecreateMoveSelector(_)
        | MoveSelectorConfig::ListRuinMoveSelector(_) => true,
        MoveSelectorConfig::LimitedNeighborhood(limit) => {
            selector_requires_score_during_move(limit.selector.as_ref())
        }
        MoveSelectorConfig::UnionMoveSelector(union) => union
            .selectors
            .iter()
            .any(selector_requires_score_during_move),
        MoveSelectorConfig::CartesianProductMoveSelector(_) => true,
        _ => false,
    }
}

fn assert_cartesian_left_preview_safe(config: &MoveSelectorConfig) {
    assert!(
        !selector_requires_score_during_move(config),
        "cartesian_product left child cannot contain ruin_recreate_move_selector or list_ruin_move_selector because preview directors do not calculate scores",
    );
}

fn push_scalar_selector<S, V, DM, IDM>(
    config: Option<&MoveSelectorConfig>,
    model: &ModelContext<S, V, DM, IDM>,
    random_seed: Option<u64>,
    out: &mut Vec<NeighborhoodLeaf<S, V, DM, IDM>>,
) where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    let scalar_variables: Vec<_> = model.scalar_variables().copied().collect();
    if scalar_variables.is_empty() {
        return;
    }
    let selector = build_scalar_flat_selector(config, &scalar_variables, random_seed);
    out.extend(
        selector
            .into_selectors()
            .into_iter()
            .map(NeighborhoodLeaf::Scalar),
    );
}

fn push_list_selector<S, V, DM, IDM>(
    config: Option<&MoveSelectorConfig>,
    model: &ModelContext<S, V, DM, IDM>,
    random_seed: Option<u64>,
    out: &mut Vec<NeighborhoodLeaf<S, V, DM, IDM>>,
) where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    for variable in model.list_variables() {
        let selector = ListMoveSelectorBuilder::build_flat(config, variable, random_seed);
        out.extend(
            selector
                .into_selectors()
                .into_iter()
                .map(NeighborhoodLeaf::List),
        );
    }
}

fn build_leaf_selector<S, V, DM, IDM>(
    config: Option<&MoveSelectorConfig>,
    model: &ModelContext<S, V, DM, IDM>,
    random_seed: Option<u64>,
) -> LeafSelector<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    let mut leaves = Vec::new();
    match config {
        None => unreachable!("default neighborhoods must be resolved before leaf selection"),
        Some(MoveSelectorConfig::ChangeMoveSelector(_))
        | Some(MoveSelectorConfig::SwapMoveSelector(_))
        | Some(MoveSelectorConfig::NearbyChangeMoveSelector(_))
        | Some(MoveSelectorConfig::NearbySwapMoveSelector(_))
        | Some(MoveSelectorConfig::PillarChangeMoveSelector(_))
        | Some(MoveSelectorConfig::PillarSwapMoveSelector(_))
        | Some(MoveSelectorConfig::RuinRecreateMoveSelector(_)) => {
            push_scalar_selector(config, model, random_seed, &mut leaves);
        }
        Some(MoveSelectorConfig::ListChangeMoveSelector(_))
        | Some(MoveSelectorConfig::NearbyListChangeMoveSelector(_))
        | Some(MoveSelectorConfig::ListSwapMoveSelector(_))
        | Some(MoveSelectorConfig::NearbyListSwapMoveSelector(_))
        | Some(MoveSelectorConfig::SublistChangeMoveSelector(_))
        | Some(MoveSelectorConfig::SublistSwapMoveSelector(_))
        | Some(MoveSelectorConfig::ListReverseMoveSelector(_))
        | Some(MoveSelectorConfig::KOptMoveSelector(_))
        | Some(MoveSelectorConfig::ListRuinMoveSelector(_)) => {
            push_list_selector(config, model, random_seed, &mut leaves);
        }
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => {
            for child in &union.selectors {
                match selector_family(child) {
                    SelectorFamily::Scalar => {
                        push_scalar_selector(Some(child), model, random_seed, &mut leaves);
                    }
                    SelectorFamily::List => {
                        push_list_selector(Some(child), model, random_seed, &mut leaves);
                    }
                    SelectorFamily::Mixed => {
                        let nested = build_leaf_selector(Some(child), model, random_seed);
                        leaves.extend(nested.into_selectors());
                    }
                    SelectorFamily::Unsupported => {
                        panic!(
                            "cartesian_product move selectors are not supported in the runtime selector graph"
                        );
                    }
                }
            }
        }
        Some(MoveSelectorConfig::LimitedNeighborhood(_)) => {
            panic!("limited_neighborhood must be wrapped at the neighborhood level");
        }
        Some(MoveSelectorConfig::CartesianProductMoveSelector(_)) => {
            panic!(
                "cartesian_product move selectors are not supported in the runtime selector graph"
            );
        }
    }
    assert!(
        !leaves.is_empty(),
        "move selector configuration produced no neighborhoods",
    );
    VecUnionSelector::new(leaves)
}

fn build_cartesian_child_selector<S, V, DM, IDM>(
    config: &MoveSelectorConfig,
    model: &ModelContext<S, V, DM, IDM>,
    random_seed: Option<u64>,
) -> CartesianChildSelector<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    match config {
        MoveSelectorConfig::LimitedNeighborhood(limit) => CartesianChildSelector::Limited {
            selector: build_leaf_selector(Some(limit.selector.as_ref()), model, random_seed),
            selected_count_limit: limit.selected_count_limit,
        },
        MoveSelectorConfig::CartesianProductMoveSelector(_) => {
            panic!("nested cartesian_product move selectors are not supported")
        }
        other => CartesianChildSelector::Flat(build_leaf_selector(Some(other), model, random_seed)),
    }
}

fn wrap_neighborhood_composite<S, V>(
    mov: SequentialCompositeMove<S, NeighborhoodMove<S, V>>,
) -> NeighborhoodMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    NeighborhoodMove::Composite(mov)
}

fn default_scalar_change_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
        target: VariableTargetConfig::default(),
    })
}

fn default_scalar_swap_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::SwapMoveSelector(solverforge_config::SwapMoveConfig {
        target: VariableTargetConfig::default(),
    })
}

fn default_nearby_list_change_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::NearbyListChangeMoveSelector(NearbyListChangeMoveConfig {
        max_nearby: 20,
        target: VariableTargetConfig::default(),
    })
}

fn default_nearby_list_swap_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::NearbyListSwapMoveSelector(NearbyListSwapMoveConfig {
        max_nearby: 20,
        target: VariableTargetConfig::default(),
    })
}

fn default_list_reverse_selector() -> MoveSelectorConfig {
    MoveSelectorConfig::ListReverseMoveSelector(ListReverseMoveConfig {
        target: VariableTargetConfig::default(),
    })
}

fn collect_default_neighborhoods<S, V, DM, IDM>(
    model: &ModelContext<S, V, DM, IDM>,
    random_seed: Option<u64>,
    out: &mut Vec<Neighborhood<S, V, DM, IDM>>,
) where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    if model.has_list_variables() {
        let list_change = default_nearby_list_change_selector();
        out.push(Neighborhood::Flat(build_leaf_selector(
            Some(&list_change),
            model,
            random_seed,
        )));

        let list_swap = default_nearby_list_swap_selector();
        out.push(Neighborhood::Flat(build_leaf_selector(
            Some(&list_swap),
            model,
            random_seed,
        )));

        let list_reverse = default_list_reverse_selector();
        out.push(Neighborhood::Flat(build_leaf_selector(
            Some(&list_reverse),
            model,
            random_seed,
        )));
    }

    if model.scalar_variables().next().is_some() {
        let scalar_change = default_scalar_change_selector();
        out.push(Neighborhood::Flat(build_leaf_selector(
            Some(&scalar_change),
            model,
            random_seed,
        )));

        let scalar_swap = default_scalar_swap_selector();
        out.push(Neighborhood::Flat(build_leaf_selector(
            Some(&scalar_swap),
            model,
            random_seed,
        )));
    }
}

fn collect_neighborhoods<S, V, DM, IDM>(
    config: Option<&MoveSelectorConfig>,
    model: &ModelContext<S, V, DM, IDM>,
    random_seed: Option<u64>,
    out: &mut Vec<Neighborhood<S, V, DM, IDM>>,
) where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    match config {
        None => collect_default_neighborhoods(model, random_seed, out),
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => {
            for child in &union.selectors {
                collect_neighborhoods(Some(child), model, random_seed, out);
            }
        }
        Some(MoveSelectorConfig::LimitedNeighborhood(limit)) => {
            let selector = build_leaf_selector(Some(limit.selector.as_ref()), model, random_seed);
            out.push(Neighborhood::Limited {
                selector,
                selected_count_limit: limit.selected_count_limit,
            });
        }
        Some(MoveSelectorConfig::CartesianProductMoveSelector(cartesian)) => {
            assert_eq!(
                cartesian.selectors.len(),
                2,
                "cartesian_product move selector requires exactly two child selectors"
            );
            assert_cartesian_left_preview_safe(&cartesian.selectors[0]);
            let left = build_cartesian_child_selector(&cartesian.selectors[0], model, random_seed);
            let right = build_cartesian_child_selector(&cartesian.selectors[1], model, random_seed);
            out.push(Neighborhood::Cartesian(CartesianProductSelector::new(
                left,
                right,
                wrap_neighborhood_composite::<S, V>,
            )));
        }
        Some(other) => out.push(Neighborhood::Flat(build_leaf_selector(
            Some(other),
            model,
            random_seed,
        ))),
    }
}

pub fn build_move_selector<S, V, DM, IDM>(
    config: Option<&MoveSelectorConfig>,
    model: &ModelContext<S, V, DM, IDM>,
    random_seed: Option<u64>,
) -> Selector<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    let mut neighborhoods = Vec::new();
    collect_neighborhoods(config, model, random_seed, &mut neighborhoods);
    assert!(
        !neighborhoods.is_empty(),
        "move selector configuration produced no neighborhoods",
    );
    VecUnionSelector::new(neighborhoods)
}

pub fn build_local_search<S, V, DM, IDM>(
    config: Option<&LocalSearchConfig>,
    model: &ModelContext<S, V, DM, IDM>,
    random_seed: Option<u64>,
) -> LocalSearch<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    let acceptor = config
        .and_then(|ls| ls.acceptor.as_ref())
        .map(|cfg| AcceptorBuilder::build_with_seed::<S>(cfg, random_seed))
        .unwrap_or_else(|| {
            if model.has_list_variables() {
                AnyAcceptor::LateAcceptance(
                    crate::phase::localsearch::LateAcceptanceAcceptor::<S>::new(400),
                )
            } else {
                match random_seed {
                    Some(seed) => AnyAcceptor::SimulatedAnnealing(
                        SimulatedAnnealingAcceptor::auto_calibrate_with_seed(0.999985, seed),
                    ),
                    None => AnyAcceptor::SimulatedAnnealing(SimulatedAnnealingAcceptor::default()),
                }
            }
        });
    let forager = config
        .and_then(|ls| ls.forager.as_ref())
        .map(|cfg| ForagerBuilder::build::<S>(Some(cfg)))
        .unwrap_or_else(|| {
            let is_tabu = config
                .and_then(|ls| ls.acceptor.as_ref())
                .is_some_and(|acceptor| matches!(acceptor, AcceptorConfig::TabuSearch(_)));
            if is_tabu {
                AnyForager::BestScore(crate::phase::localsearch::BestScoreForager::new())
            } else {
                let accepted = if model.has_list_variables() { 4 } else { 1 };
                AnyForager::AcceptedCount(AcceptedCountForager::new(accepted))
            }
        });
    let move_selector = build_move_selector(
        config.and_then(|ls| ls.move_selector.as_ref()),
        model,
        random_seed,
    );

    LocalSearchPhase::new(move_selector, acceptor, forager, None)
}

pub fn build_vnd<S, V, DM, IDM>(
    config: &VndConfig,
    model: &ModelContext<S, V, DM, IDM>,
    random_seed: Option<u64>,
) -> Vnd<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    let neighborhoods = if config.neighborhoods.is_empty() {
        let mut neighborhoods = Vec::new();
        collect_neighborhoods(None, model, random_seed, &mut neighborhoods);
        neighborhoods
    } else {
        config
            .neighborhoods
            .iter()
            .flat_map(|selector| {
                let mut neighborhoods = Vec::new();
                collect_neighborhoods(Some(selector), model, random_seed, &mut neighborhoods);
                neighborhoods
            })
            .collect()
    };

    DynamicVndPhase::new(neighborhoods)
}

#[cfg(test)]
mod tests;
