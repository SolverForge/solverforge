use std::fmt::{self, Debug};

use solverforge_config::{LocalSearchConfig, MoveSelectorConfig, VndConfig};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};

use crate::heuristic::r#move::{EitherMove, ListMoveImpl, Move, MoveArena};
use crate::heuristic::selector::decorator::{SelectedCountLimitMoveSelector, VecUnionSelector};
use crate::heuristic::selector::move_selector::MoveSelector;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::dynamic_vnd::DynamicVndPhase;
use crate::phase::localsearch::{
    AcceptedCountForager, LocalSearchPhase, SimulatedAnnealingAcceptor,
};

use super::acceptor::{AcceptorBuilder, AnyAcceptor};
use super::context::ModelContext;
use super::forager::{AnyForager, ForagerBuilder};
use super::list_selector::{ListLeafSelector, ListMoveSelectorBuilder};
use super::standard_selector::{build_standard_move_selector, StandardLeafSelector};

type LeafSelector<S, V, DM, IDM> =
    VecUnionSelector<S, NeighborhoodMove<S, V>, NeighborhoodLeaf<S, V, DM, IDM>>;

type LimitedNeighborhood<S, V, DM, IDM> =
    SelectedCountLimitMoveSelector<S, NeighborhoodMove<S, V>, LeafSelector<S, V, DM, IDM>>;

pub enum NeighborhoodMove<S, V> {
    Scalar(EitherMove<S, usize>),
    List(ListMoveImpl<S, V>),
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
        }
    }

    fn do_move<D: solverforge_scoring::Director<S>>(&self, score_director: &mut D) {
        match self {
            Self::Scalar(m) => m.do_move(score_director),
            Self::List(m) => m.do_move(score_director),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Scalar(m) => m.descriptor_index(),
            Self::List(m) => m.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Scalar(m) => m.entity_indices(),
            Self::List(m) => m.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Scalar(m) => m.variable_name(),
            Self::List(m) => m.variable_name(),
        }
    }
}

pub enum NeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone,
{
    Scalar(StandardLeafSelector<S>),
    List(ListLeafSelector<S, V, DM, IDM>),
}

impl<S, V, DM, IDM> Debug for NeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug,
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
    fn open_cursor<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = NeighborhoodMove<S, V>> + 'a {
        enum LeafIter<A, B> {
            Scalar(A),
            List(B),
        }

        impl<T, A, B> Iterator for LeafIter<A, B>
        where
            A: Iterator<Item = T>,
            B: Iterator<Item = T>,
        {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Self::Scalar(iter) => iter.next(),
                    Self::List(iter) => iter.next(),
                }
            }
        }

        match self {
            Self::Scalar(selector) => LeafIter::Scalar(
                selector
                    .open_cursor(score_director)
                    .map(NeighborhoodMove::Scalar),
            ),
            Self::List(selector) => LeafIter::List(
                selector
                    .open_cursor(score_director)
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
        match self {
            Self::Scalar(selector) => {
                arena.extend(
                    selector
                        .open_cursor(score_director)
                        .map(NeighborhoodMove::Scalar),
                );
            }
            Self::List(selector) => {
                arena.extend(
                    selector
                        .open_cursor(score_director)
                        .map(NeighborhoodMove::List),
                );
            }
        }
    }
}

pub enum Neighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone,
{
    Flat(LeafSelector<S, V, DM, IDM>),
    Limited(LimitedNeighborhood<S, V, DM, IDM>),
}

impl<S, V, DM, IDM> Debug for Neighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flat(selector) => write!(f, "Neighborhood::Flat({selector:?})"),
            Self::Limited(selector) => write!(f, "Neighborhood::Limited({selector:?})"),
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
    fn open_cursor<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = NeighborhoodMove<S, V>> + 'a {
        enum NeighborhoodIter<A, B> {
            Flat(A),
            Limited(B),
        }

        impl<T, A, B> Iterator for NeighborhoodIter<A, B>
        where
            A: Iterator<Item = T>,
            B: Iterator<Item = T>,
        {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Self::Flat(iter) => iter.next(),
                    Self::Limited(iter) => iter.next(),
                }
            }
        }

        match self {
            Self::Flat(selector) => NeighborhoodIter::Flat(selector.open_cursor(score_director)),
            Self::Limited(selector) => {
                NeighborhoodIter::Limited(selector.open_cursor(score_director))
            }
        }
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Flat(selector) => selector.size(score_director),
            Self::Limited(selector) => selector.size(score_director),
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
        MoveSelectorConfig::ChangeMoveSelector(_) | MoveSelectorConfig::SwapMoveSelector(_) => {
            SelectorFamily::Scalar
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

fn push_scalar_selector<S, V, DM, IDM>(
    config: Option<&MoveSelectorConfig>,
    model: &ModelContext<S, V, DM, IDM>,
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
    let selector = build_standard_move_selector(config, &scalar_variables);
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
        let selector = ListMoveSelectorBuilder::build(config, variable, random_seed);
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
        None => {
            push_scalar_selector(None, model, &mut leaves);
            push_list_selector(None, model, random_seed, &mut leaves);
        }
        Some(MoveSelectorConfig::ChangeMoveSelector(_))
        | Some(MoveSelectorConfig::SwapMoveSelector(_)) => {
            push_scalar_selector(config, model, &mut leaves);
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
            push_list_selector(config, model, random_seed, &mut leaves);
        }
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => {
            for child in &union.selectors {
                match selector_family(child) {
                    SelectorFamily::Scalar => {
                        push_scalar_selector(Some(child), model, &mut leaves);
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
        Some(MoveSelectorConfig::SelectedCountLimitMoveSelector(_)) => {
            panic!("selected_count_limit_move_selector must be wrapped at the neighborhood level");
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
        None => out.push(Neighborhood::Flat(build_leaf_selector(
            None,
            model,
            random_seed,
        ))),
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => {
            for child in &union.selectors {
                collect_neighborhoods(Some(child), model, random_seed, out);
            }
        }
        Some(MoveSelectorConfig::SelectedCountLimitMoveSelector(limit)) => {
            let selector = build_leaf_selector(Some(limit.selector.as_ref()), model, random_seed);
            out.push(Neighborhood::Limited(SelectedCountLimitMoveSelector::new(
                selector,
                limit.selected_count_limit,
            )));
        }
        Some(MoveSelectorConfig::CartesianProductMoveSelector(_)) => {
            panic!(
                "cartesian_product move selectors are not supported in the runtime selector graph"
            );
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
            let accepted = if model.has_list_variables() { 4 } else { 1 };
            AnyForager::AcceptedCount(AcceptedCountForager::new(accepted))
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
