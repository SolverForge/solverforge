use std::fmt::{self, Debug};

use solverforge_config::{LocalSearchConfig, MoveSelectorConfig, VndConfig};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};

use crate::builder::{
    build_standard_move_selector, AcceptorBuilder, AnyAcceptor, AnyForager, ForagerBuilder,
    ListContext, ListLeafSelector, ListMoveSelectorBuilder, StandardContext, StandardSelector,
};
use crate::heuristic::r#move::{EitherMove, ListMoveImpl, Move, MoveArena};
use crate::heuristic::selector::decorator::{SelectedCountLimitMoveSelector, VecUnionSelector};
use crate::heuristic::selector::move_selector::MoveSelector;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::dynamic_vnd::DynamicVndPhase;
use crate::phase::localsearch::{
    AcceptedCountForager, LocalSearchPhase, SimulatedAnnealingAcceptor,
};

type LimitedStandardSelector<S> =
    SelectedCountLimitMoveSelector<S, EitherMove<S, usize>, StandardSelector<S>>;
type ListSelector<S, V, DM, IDM> =
    VecUnionSelector<S, ListMoveImpl<S, V>, ListLeafSelector<S, V, DM, IDM>>;
type LimitedListSelector<S, V, DM, IDM> =
    SelectedCountLimitMoveSelector<S, ListMoveImpl<S, V>, ListSelector<S, V, DM, IDM>>;

pub enum UnifiedMove<S, V> {
    Standard(EitherMove<S, usize>),
    List(ListMoveImpl<S, V>),
}

impl<S, V> Debug for UnifiedMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Standard(m) => write!(f, "UnifiedMove::Standard({m:?})"),
            Self::List(m) => write!(f, "UnifiedMove::List({m:?})"),
        }
    }
}

impl<S, V> Move<S> for UnifiedMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::Standard(m) => m.is_doable(score_director),
            Self::List(m) => m.is_doable(score_director),
        }
    }

    fn do_move<D: solverforge_scoring::Director<S>>(&self, score_director: &mut D) {
        match self {
            Self::Standard(m) => m.do_move(score_director),
            Self::List(m) => m.do_move(score_director),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Standard(m) => m.descriptor_index(),
            Self::List(m) => m.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Standard(m) => m.entity_indices(),
            Self::List(m) => m.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Standard(m) => m.variable_name(),
            Self::List(m) => m.variable_name(),
        }
    }
}

pub enum UnifiedNeighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone,
{
    Standard(StandardSelector<S>),
    LimitedStandard(LimitedStandardSelector<S>),
    List(ListSelector<S, V, DM, IDM>),
    LimitedList(LimitedListSelector<S, V, DM, IDM>),
}

impl<S, V, DM, IDM> Debug for UnifiedNeighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Standard(s) => write!(f, "UnifiedNeighborhood::Standard({s:?})"),
            Self::LimitedStandard(s) => write!(f, "UnifiedNeighborhood::LimitedStandard({s:?})"),
            Self::List(s) => write!(f, "UnifiedNeighborhood::List({s:?})"),
            Self::LimitedList(s) => write!(f, "UnifiedNeighborhood::LimitedList({s:?})"),
        }
    }
}

impl<S, V, DM, IDM> MoveSelector<S, UnifiedMove<S, V>> for UnifiedNeighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn open_cursor<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = UnifiedMove<S, V>> + 'a {
        enum UnifiedNeighborhoodIter<A, B, C, DIter> {
            Standard(A),
            LimitedStandard(B),
            List(C),
            LimitedList(DIter),
        }

        impl<T, A, B, C, DIter> Iterator for UnifiedNeighborhoodIter<A, B, C, DIter>
        where
            A: Iterator<Item = T>,
            B: Iterator<Item = T>,
            C: Iterator<Item = T>,
            DIter: Iterator<Item = T>,
        {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Self::Standard(iter) => iter.next(),
                    Self::LimitedStandard(iter) => iter.next(),
                    Self::List(iter) => iter.next(),
                    Self::LimitedList(iter) => iter.next(),
                }
            }
        }

        match self {
            Self::Standard(selector) => UnifiedNeighborhoodIter::Standard(
                selector
                    .open_cursor(score_director)
                    .map(UnifiedMove::Standard),
            ),
            Self::LimitedStandard(selector) => UnifiedNeighborhoodIter::LimitedStandard(
                selector
                    .open_cursor(score_director)
                    .map(UnifiedMove::Standard),
            ),
            Self::List(selector) => UnifiedNeighborhoodIter::List(
                selector.open_cursor(score_director).map(UnifiedMove::List),
            ),
            Self::LimitedList(selector) => UnifiedNeighborhoodIter::LimitedList(
                selector.open_cursor(score_director).map(UnifiedMove::List),
            ),
        }
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Standard(selector) => selector.size(score_director),
            Self::LimitedStandard(selector) => selector.size(score_director),
            Self::List(selector) => selector.size(score_director),
            Self::LimitedList(selector) => selector.size(score_director),
        }
    }

    fn append_moves<D: solverforge_scoring::Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<UnifiedMove<S, V>>,
    ) {
        match self {
            Self::Standard(selector) => {
                for mov in selector.open_cursor(score_director) {
                    arena.push(UnifiedMove::Standard(mov));
                }
            }
            Self::LimitedStandard(selector) => {
                for mov in selector.open_cursor(score_director) {
                    arena.push(UnifiedMove::Standard(mov));
                }
            }
            Self::List(selector) => {
                for mov in selector.open_cursor(score_director) {
                    arena.push(UnifiedMove::List(mov));
                }
            }
            Self::LimitedList(selector) => {
                for mov in selector.open_cursor(score_director) {
                    arena.push(UnifiedMove::List(mov));
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectorFamily {
    Standard,
    List,
    Mixed,
    Unsupported,
}

fn selector_family(config: &MoveSelectorConfig) -> SelectorFamily {
    match config {
        MoveSelectorConfig::ChangeMoveSelector(_) | MoveSelectorConfig::SwapMoveSelector(_) => {
            SelectorFamily::Standard
        }
        MoveSelectorConfig::ListChangeMoveSelector(_)
        | MoveSelectorConfig::NearbyListChangeMoveSelector(_)
        | MoveSelectorConfig::ListSwapMoveSelector(_)
        | MoveSelectorConfig::NearbyListSwapMoveSelector(_)
        | MoveSelectorConfig::SubListChangeMoveSelector(_)
        | MoveSelectorConfig::SubListSwapMoveSelector(_)
        | MoveSelectorConfig::ListReverseMoveSelector(_)
        | MoveSelectorConfig::KOptMoveSelector(_)
        | MoveSelectorConfig::ListRuinMoveSelector(_) => SelectorFamily::List,
        MoveSelectorConfig::SelectedCountLimitMoveSelector(limit) => {
            selector_family(limit.selector.as_ref())
        }
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

pub type UnifiedLocalSearch<S, V, DM, IDM> = LocalSearchPhase<
    S,
    UnifiedMove<S, V>,
    VecUnionSelector<S, UnifiedMove<S, V>, UnifiedNeighborhood<S, V, DM, IDM>>,
    AnyAcceptor<S>,
    AnyForager<S>,
>;

pub type UnifiedVnd<S, V, DM, IDM> =
    DynamicVndPhase<S, UnifiedMove<S, V>, UnifiedNeighborhood<S, V, DM, IDM>>;

pub fn build_unified_move_selector<S, V, DM, IDM>(
    config: Option<&MoveSelectorConfig>,
    descriptor: &SolutionDescriptor,
    standard_ctx: Option<&StandardContext<S>>,
    list_ctx: Option<&ListContext<S, V, DM, IDM>>,
    random_seed: Option<u64>,
) -> VecUnionSelector<S, UnifiedMove<S, V>, UnifiedNeighborhood<S, V, DM, IDM>>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    let mut neighborhoods = Vec::new();
    collect_neighborhoods(
        config,
        descriptor,
        standard_ctx,
        list_ctx,
        random_seed,
        &mut neighborhoods,
    );
    assert!(
        !neighborhoods.is_empty(),
        "stock move selector configuration produced no neighborhoods"
    );
    VecUnionSelector::new(neighborhoods)
}

fn collect_neighborhoods<S, V, DM, IDM>(
    config: Option<&MoveSelectorConfig>,
    descriptor: &SolutionDescriptor,
    standard_ctx: Option<&StandardContext<S>>,
    list_ctx: Option<&ListContext<S, V, DM, IDM>>,
    random_seed: Option<u64>,
    out: &mut Vec<UnifiedNeighborhood<S, V, DM, IDM>>,
) where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    match config {
        None => {
            if let Some(standard_ctx) = standard_ctx.filter(|ctx| !ctx.is_empty()) {
                out.push(UnifiedNeighborhood::Standard(build_standard_move_selector(
                    None,
                    standard_ctx,
                )));
            }
            if let Some(list_ctx) = list_ctx {
                out.push(UnifiedNeighborhood::List(ListMoveSelectorBuilder::build(
                    None,
                    list_ctx,
                    random_seed,
                )));
            }
        }
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => {
            for child in &union.selectors {
                collect_neighborhoods(
                    Some(child),
                    descriptor,
                    standard_ctx,
                    list_ctx,
                    random_seed,
                    out,
                );
            }
        }
        Some(MoveSelectorConfig::SelectedCountLimitMoveSelector(limit)) => {
            match selector_family(limit.selector.as_ref()) {
                SelectorFamily::Standard => {
                    let Some(standard_ctx) = standard_ctx.filter(|ctx| !ctx.is_empty()) else {
                        panic!(
                            "selected_count_limit_move_selector wrapped a standard selector in a list-only stock context"
                        );
                    };
                    out.push(UnifiedNeighborhood::LimitedStandard(
                        SelectedCountLimitMoveSelector::new(
                            build_standard_move_selector(
                                Some(limit.selector.as_ref()),
                                standard_ctx,
                            ),
                            limit.selected_count_limit,
                        ),
                    ))
                }
                SelectorFamily::List => {
                    let Some(list_ctx) = list_ctx else {
                        panic!(
                            "selected_count_limit_move_selector wrapped a list selector in a standard-variable stock context"
                        );
                    };
                    out.push(UnifiedNeighborhood::LimitedList(
                        SelectedCountLimitMoveSelector::new(
                            ListMoveSelectorBuilder::build(
                                Some(limit.selector.as_ref()),
                                list_ctx,
                                random_seed,
                            ),
                            limit.selected_count_limit,
                        ),
                    ));
                }
                SelectorFamily::Mixed => {
                    panic!(
                        "selected_count_limit_move_selector cannot wrap a mixed standard/list selector union"
                    );
                }
                SelectorFamily::Unsupported => {
                    panic!("cartesian_product move selectors are not supported in stock solving");
                }
            }
        }
        Some(MoveSelectorConfig::ChangeMoveSelector(_))
        | Some(MoveSelectorConfig::SwapMoveSelector(_)) => {
            let Some(standard_ctx) = standard_ctx.filter(|ctx| !ctx.is_empty()) else {
                panic!("standard move selector configured against a list-only stock context");
            };
            out.push(UnifiedNeighborhood::Standard(build_standard_move_selector(
                config,
                standard_ctx,
            )));
        }
        Some(MoveSelectorConfig::ListChangeMoveSelector(_))
        | Some(MoveSelectorConfig::NearbyListChangeMoveSelector(_))
        | Some(MoveSelectorConfig::ListSwapMoveSelector(_))
        | Some(MoveSelectorConfig::NearbyListSwapMoveSelector(_))
        | Some(MoveSelectorConfig::SubListChangeMoveSelector(_))
        | Some(MoveSelectorConfig::SubListSwapMoveSelector(_))
        | Some(MoveSelectorConfig::ListReverseMoveSelector(_))
        | Some(MoveSelectorConfig::KOptMoveSelector(_))
        | Some(MoveSelectorConfig::ListRuinMoveSelector(_)) => {
            let Some(list_ctx) = list_ctx else {
                panic!("list move selector configured against a standard-variable stock context");
            };
            out.push(UnifiedNeighborhood::List(ListMoveSelectorBuilder::build(
                config,
                list_ctx,
                random_seed,
            )));
        }
        Some(MoveSelectorConfig::CartesianProductMoveSelector(_)) => {
            panic!("cartesian_product move selectors are not supported in stock solving");
        }
    }
}

pub fn build_unified_local_search<S, V, DM, IDM>(
    config: Option<&LocalSearchConfig>,
    descriptor: &SolutionDescriptor,
    standard_ctx: Option<&StandardContext<S>>,
    list_ctx: Option<&ListContext<S, V, DM, IDM>>,
    random_seed: Option<u64>,
) -> UnifiedLocalSearch<S, V, DM, IDM>
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
            if list_ctx.is_some() {
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
            let accepted = if list_ctx.is_some() { 4 } else { 1 };
            AnyForager::AcceptedCount(AcceptedCountForager::new(accepted))
        });
    let move_selector = build_unified_move_selector(
        config.and_then(|ls| ls.move_selector.as_ref()),
        descriptor,
        standard_ctx,
        list_ctx,
        random_seed,
    );

    LocalSearchPhase::new(move_selector, acceptor, forager, None)
}

pub fn build_unified_vnd<S, V, DM, IDM>(
    config: &VndConfig,
    descriptor: &SolutionDescriptor,
    standard_ctx: Option<&StandardContext<S>>,
    list_ctx: Option<&ListContext<S, V, DM, IDM>>,
    random_seed: Option<u64>,
) -> UnifiedVnd<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    let neighborhoods = if config.neighborhoods.is_empty() {
        let mut neighborhoods = Vec::new();
        collect_neighborhoods(
            None,
            descriptor,
            standard_ctx,
            list_ctx,
            random_seed,
            &mut neighborhoods,
        );
        neighborhoods
    } else {
        config
            .neighborhoods
            .iter()
            .flat_map(|selector| {
                let mut neighborhoods = Vec::new();
                collect_neighborhoods(
                    Some(selector),
                    descriptor,
                    standard_ctx,
                    list_ctx,
                    random_seed,
                    &mut neighborhoods,
                );
                neighborhoods
            })
            .collect()
    };

    DynamicVndPhase::new(neighborhoods)
}
