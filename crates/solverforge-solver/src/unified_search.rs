use std::fmt::{self, Debug};

use solverforge_config::{LocalSearchConfig, MoveSelectorConfig, VndConfig};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};

use crate::builder::{
    AcceptorBuilder, AnyAcceptor, AnyForager, ForagerBuilder, ListContext, ListLeafSelector,
    ListMoveSelectorBuilder,
};
use crate::descriptor_standard::{
    build_descriptor_move_selector, descriptor_has_bindings, DescriptorEitherMove,
    DescriptorLeafSelector,
};
use crate::heuristic::r#move::{ListMoveImpl, Move, MoveArena};
use crate::heuristic::selector::decorator::VecUnionSelector;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::typed_move_selector::MoveSelector;
use crate::phase::dynamic_vnd::DynamicVndPhase;
use crate::phase::localsearch::{
    AcceptedCountForager, LocalSearchPhase, SimulatedAnnealingAcceptor,
};

pub enum UnifiedMove<S, V> {
    Standard(DescriptorEitherMove<S>),
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
    Standard(VecUnionSelector<S, DescriptorEitherMove<S>, DescriptorLeafSelector<S>>),
    List(VecUnionSelector<S, ListMoveImpl<S, V>, ListLeafSelector<S, V, DM, IDM>>),
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
            Self::List(s) => write!(f, "UnifiedNeighborhood::List({s:?})"),
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
    fn iter_moves<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = UnifiedMove<S, V>> + 'a {
        let moves: Vec<_> = match self {
            Self::Standard(selector) => selector
                .iter_moves(score_director)
                .map(UnifiedMove::Standard)
                .collect(),
            Self::List(selector) => selector
                .iter_moves(score_director)
                .map(UnifiedMove::List)
                .collect(),
        };
        moves.into_iter()
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Standard(selector) => selector.size(score_director),
            Self::List(selector) => selector.size(score_director),
        }
    }

    fn append_moves<D: solverforge_scoring::Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<UnifiedMove<S, V>>,
    ) {
        match self {
            Self::Standard(selector) => {
                for mov in selector.iter_moves(score_director) {
                    arena.push(UnifiedMove::Standard(mov));
                }
            }
            Self::List(selector) => {
                for mov in selector.iter_moves(score_director) {
                    arena.push(UnifiedMove::List(mov));
                }
            }
        }
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
    list_ctx: Option<&ListContext<S, V, DM, IDM>>,
) -> VecUnionSelector<S, UnifiedMove<S, V>, UnifiedNeighborhood<S, V, DM, IDM>>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    let mut neighborhoods = Vec::new();
    collect_neighborhoods(config, descriptor, list_ctx, &mut neighborhoods);
    assert!(
        !neighborhoods.is_empty(),
        "stock move selector configuration produced no neighborhoods"
    );
    VecUnionSelector::new(neighborhoods)
}

fn collect_neighborhoods<S, V, DM, IDM>(
    config: Option<&MoveSelectorConfig>,
    descriptor: &SolutionDescriptor,
    list_ctx: Option<&ListContext<S, V, DM, IDM>>,
    out: &mut Vec<UnifiedNeighborhood<S, V, DM, IDM>>,
) where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    match config {
        None => {
            if descriptor_has_bindings(descriptor) {
                out.push(UnifiedNeighborhood::Standard(
                    build_descriptor_move_selector(None, descriptor),
                ));
            }
            if let Some(list_ctx) = list_ctx {
                out.push(UnifiedNeighborhood::List(ListMoveSelectorBuilder::build(
                    None, list_ctx,
                )));
            }
        }
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => {
            for child in &union.selectors {
                collect_neighborhoods(Some(child), descriptor, list_ctx, out);
            }
        }
        Some(MoveSelectorConfig::ChangeMoveSelector(_))
        | Some(MoveSelectorConfig::SwapMoveSelector(_)) => {
            out.push(UnifiedNeighborhood::Standard(
                build_descriptor_move_selector(config, descriptor),
            ));
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
                config, list_ctx,
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
    list_ctx: Option<&ListContext<S, V, DM, IDM>>,
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
        .map(AcceptorBuilder::build::<S>)
        .unwrap_or_else(|| {
            if list_ctx.is_some() {
                AnyAcceptor::LateAcceptance(
                    crate::phase::localsearch::LateAcceptanceAcceptor::<S>::new(400),
                )
            } else {
                AnyAcceptor::SimulatedAnnealing(SimulatedAnnealingAcceptor::default())
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
        list_ctx,
    );

    LocalSearchPhase::new(move_selector, acceptor, forager, None)
}

pub fn build_unified_vnd<S, V, DM, IDM>(
    config: &VndConfig,
    descriptor: &SolutionDescriptor,
    list_ctx: Option<&ListContext<S, V, DM, IDM>>,
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
        collect_neighborhoods(None, descriptor, list_ctx, &mut neighborhoods);
        neighborhoods
    } else {
        config
            .neighborhoods
            .iter()
            .flat_map(|selector| {
                let mut neighborhoods = Vec::new();
                collect_neighborhoods(Some(selector), descriptor, list_ctx, &mut neighborhoods);
                neighborhoods
            })
            .collect()
    };

    DynamicVndPhase::new(neighborhoods)
}
