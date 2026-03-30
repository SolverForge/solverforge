/* List stock helpers for routing and scheduling problems.

This module provides the concrete list construction and local-search
building blocks used by the unified stock runtime. The solver
configuration (construction type, move selectors, acceptor, forager,
termination) is driven by `solver.toml`.

Logging levels:
- **INFO**: Solver start/end, phase summaries, problem scale
- **DEBUG**: Individual steps with timing and scores
- **TRACE**: Move evaluation details
*/

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, PhaseConfig, SolverConfig,
};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};
use std::fmt;
use std::marker::PhantomData;

use crate::builder::list_selector::ListLeafSelector;
use crate::builder::{
    AcceptorBuilder, AnyAcceptor, AnyForager, ForagerBuilder, ListContext, ListMoveSelectorBuilder,
};
use crate::heuristic::r#move::ListMoveImpl;
use crate::heuristic::selector::decorator::VecUnionSelector;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::manager::{
    ListCheapestInsertionPhase, ListClarkeWrightPhase, ListKOptPhase, ListRegretInsertionPhase,
};
use crate::phase::localsearch::{AcceptedCountForager, LateAcceptanceAcceptor, LocalSearchPhase};

/// Hidden stock list metadata emitted by `#[planning_entity]` and consumed by
/// macro-generated stock solve code.
pub struct StockListVariableMetadata<S, DM, IDM> {
    pub variable_name: &'static str,
    pub element_collection: &'static str,
    pub cross_distance_meter: DM,
    pub intra_distance_meter: IDM,
    pub merge_feasible_fn: Option<fn(&S, &[usize]) -> bool>,
    pub cw_depot_fn: Option<fn(&S) -> usize>,
    pub cw_distance_fn: Option<fn(&S, usize, usize) -> i64>,
    pub cw_element_load_fn: Option<fn(&S, usize) -> i64>,
    pub cw_capacity_fn: Option<fn(&S) -> i64>,
    pub cw_assign_route_fn: Option<fn(&mut S, usize, Vec<usize>)>,
    pub k_opt_get_route: Option<fn(&S, usize) -> Vec<usize>>,
    pub k_opt_set_route: Option<fn(&mut S, usize, Vec<usize>)>,
    pub k_opt_depot_fn: Option<fn(&S, usize) -> usize>,
    pub k_opt_distance_fn: Option<fn(&S, usize, usize) -> i64>,
    pub k_opt_feasible_fn: Option<fn(&S, usize, &[usize]) -> bool>,
    _phantom: PhantomData<fn() -> S>,
}

/// Hidden trait implemented by `#[planning_entity]` for list-stock entities so
/// `#[planning_solution]` can consume typed list metadata without re-stating
/// it on the solution.
pub trait StockListEntity<S> {
    type CrossDistanceMeter: CrossEntityDistanceMeter<S> + Clone + fmt::Debug;
    type IntraDistanceMeter: CrossEntityDistanceMeter<S> + Clone + fmt::Debug + 'static;

    const STOCK_LIST_VARIABLE_COUNT: usize;
    const STOCK_LIST_VARIABLE_NAME: &'static str;
    const STOCK_LIST_ELEMENT_COLLECTION: &'static str;

    fn list_field(entity: &Self) -> &[usize];
    fn list_field_mut(entity: &mut Self) -> &mut Vec<usize>;
    fn stock_list_metadata(
    ) -> StockListVariableMetadata<S, Self::CrossDistanceMeter, Self::IntraDistanceMeter>;
}

impl<S, DM, IDM> StockListVariableMetadata<S, DM, IDM> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        variable_name: &'static str,
        element_collection: &'static str,
        cross_distance_meter: DM,
        intra_distance_meter: IDM,
        merge_feasible_fn: Option<fn(&S, &[usize]) -> bool>,
        cw_depot_fn: Option<fn(&S) -> usize>,
        cw_distance_fn: Option<fn(&S, usize, usize) -> i64>,
        cw_element_load_fn: Option<fn(&S, usize) -> i64>,
        cw_capacity_fn: Option<fn(&S) -> i64>,
        cw_assign_route_fn: Option<fn(&mut S, usize, Vec<usize>)>,
        k_opt_get_route: Option<fn(&S, usize) -> Vec<usize>>,
        k_opt_set_route: Option<fn(&mut S, usize, Vec<usize>)>,
        k_opt_depot_fn: Option<fn(&S, usize) -> usize>,
        k_opt_distance_fn: Option<fn(&S, usize, usize) -> i64>,
        k_opt_feasible_fn: Option<fn(&S, usize, &[usize]) -> bool>,
    ) -> Self {
        Self {
            variable_name,
            element_collection,
            cross_distance_meter,
            intra_distance_meter,
            merge_feasible_fn,
            cw_depot_fn,
            cw_distance_fn,
            cw_element_load_fn,
            cw_capacity_fn,
            cw_assign_route_fn,
            k_opt_get_route,
            k_opt_set_route,
            k_opt_depot_fn,
            k_opt_distance_fn,
            k_opt_feasible_fn,
            _phantom: PhantomData,
        }
    }
}

// Type alias for the config-driven list local search phase
type ConfigListLocalSearch<S, V, DM, IDM> = LocalSearchPhase<
    S,
    ListMoveImpl<S, V>,
    VecUnionSelector<S, ListMoveImpl<S, V>, ListLeafSelector<S, V, DM, IDM>>,
    AnyAcceptor<S>,
    AnyForager<S>,
>;

// Type alias for the default list local search phase
type DefaultListLocalSearch<S, V, DM, IDM> = LocalSearchPhase<
    S,
    ListMoveImpl<S, V>,
    VecUnionSelector<S, ListMoveImpl<S, V>, ListLeafSelector<S, V, DM, IDM>>,
    LateAcceptanceAcceptor<S>,
    AcceptedCountForager<S>,
>;

// Monomorphized local search enum for list solver.
// Variants intentionally differ in size; this enum is constructed once per solve, not in hot paths.
#[allow(clippy::large_enum_variant)]
pub enum ListLocalSearch<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: CrossEntityDistanceMeter<S>,
    IDM: CrossEntityDistanceMeter<S> + 'static,
{
    Default(DefaultListLocalSearch<S, V, DM, IDM>),
    Config(ConfigListLocalSearch<S, V, DM, IDM>),
}

impl<S, V, DM, IDM> fmt::Debug for ListLocalSearch<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + fmt::Debug,
    IDM: CrossEntityDistanceMeter<S> + fmt::Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default(phase) => write!(f, "ListLocalSearch::Default({phase:?})"),
            Self::Config(phase) => write!(f, "ListLocalSearch::Config({phase:?})"),
        }
    }
}

// Construction phase enum for list solver.
pub enum ListConstruction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
{
    CheapestInsertion(ListCheapestInsertionPhase<S, V>),
    RegretInsertion(ListRegretInsertionPhase<S, V>),
    ClarkeWright(ListClarkeWrightPhase<S, V>),
    KOpt(ListKOptPhase<S, V>),
}

impl<S, V> fmt::Debug for ListConstruction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + fmt::Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CheapestInsertion(phase) => {
                write!(f, "ListConstruction::CheapestInsertion({phase:?})")
            }
            Self::RegretInsertion(phase) => {
                write!(f, "ListConstruction::RegretInsertion({phase:?})")
            }
            Self::ClarkeWright(phase) => {
                write!(f, "ListConstruction::ClarkeWright({phase:?})")
            }
            Self::KOpt(phase) => write!(f, "ListConstruction::KOpt({phase:?})"),
        }
    }
}

impl<S, V, D, ProgressCb, DM, IDM> crate::phase::Phase<S, D, ProgressCb>
    for ListLocalSearch<S, V, DM, IDM>
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + fmt::Debug + 'static,
    D: solverforge_scoring::Director<S>,
    ProgressCb: crate::scope::ProgressCallback<S>,
    DM: CrossEntityDistanceMeter<S> + Clone + fmt::Debug,
    IDM: CrossEntityDistanceMeter<S> + Clone + fmt::Debug + 'static,
{
    fn solve(&mut self, solver_scope: &mut crate::scope::SolverScope<'_, S, D, ProgressCb>) {
        match self {
            Self::Default(phase) => phase.solve(solver_scope),
            Self::Config(phase) => phase.solve(solver_scope),
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "ListLocalSearch"
    }
}

impl<S, V, D, ProgressCb> crate::phase::Phase<S, D, ProgressCb> for ListConstruction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + fmt::Debug + 'static,
    D: solverforge_scoring::Director<S>,
    ProgressCb: crate::scope::ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut crate::scope::SolverScope<'_, S, D, ProgressCb>) {
        match self {
            Self::CheapestInsertion(phase) => phase.solve(solver_scope),
            Self::RegretInsertion(phase) => phase.solve(solver_scope),
            Self::ClarkeWright(phase) => phase.solve(solver_scope),
            Self::KOpt(phase) => phase.solve(solver_scope),
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "ListConstruction"
    }
}

// Builds the construction phase from config or defaults to cheapest insertion.
#[allow(clippy::too_many_arguments)]
pub fn build_list_construction<S, V>(
    config: Option<&ConstructionHeuristicConfig>,
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<V>,
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_insert: fn(&mut S, usize, usize, V),
    list_remove: fn(&mut S, usize, usize) -> V,
    index_to_element: fn(&S, usize) -> V,
    descriptor_index: usize,
    depot_fn: Option<fn(&S) -> usize>,
    distance_fn: Option<fn(&S, usize, usize) -> i64>,
    element_load_fn: Option<fn(&S, usize) -> i64>,
    capacity_fn: Option<fn(&S) -> i64>,
    assign_route_fn: Option<fn(&mut S, usize, Vec<V>)>,
    merge_feasible_fn: Option<fn(&S, &[usize]) -> bool>,
    k_opt_get_route: Option<fn(&S, usize) -> Vec<usize>>,
    k_opt_set_route: Option<fn(&mut S, usize, Vec<usize>)>,
    k_opt_depot_fn: Option<fn(&S, usize) -> usize>,
    k_opt_distance_fn: Option<fn(&S, usize, usize) -> i64>,
    k_opt_feasible_fn: Option<fn(&S, usize, &[usize]) -> bool>,
) -> ListConstruction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + fmt::Debug + 'static,
{
    let (ch_type, k) = config
        .map(|cfg| (cfg.construction_heuristic_type, cfg.k))
        .unwrap_or((ConstructionHeuristicType::ListCheapestInsertion, 2));

    match ch_type {
        ConstructionHeuristicType::ListRegretInsertion => {
            ListConstruction::RegretInsertion(ListRegretInsertionPhase::new(
                element_count,
                get_assigned,
                entity_count,
                list_len,
                list_insert,
                list_remove,
                index_to_element,
                descriptor_index,
            ))
        }
        ConstructionHeuristicType::ListClarkeWright => {
            match (
                depot_fn,
                distance_fn,
                element_load_fn,
                capacity_fn,
                assign_route_fn,
            ) {
                (Some(depot), Some(dist), Some(load), Some(cap), Some(assign)) => {
                    ListConstruction::ClarkeWright(ListClarkeWrightPhase::new(
                        element_count,
                        get_assigned,
                        entity_count,
                        assign,
                        index_to_element,
                        depot,
                        dist,
                        load,
                        cap,
                        merge_feasible_fn,
                        descriptor_index,
                    ))
                }
                _ => {
                    panic!(
                        "list_clarke_wright requires depot_fn, distance_fn, \
                         element_load_fn, capacity_fn, and assign_route_fn"
                    );
                }
            }
        }
        ConstructionHeuristicType::ListKOpt => {
            match (
                k_opt_get_route,
                k_opt_set_route,
                k_opt_depot_fn,
                k_opt_distance_fn,
            ) {
                (Some(get_route), Some(set_route), Some(ko_depot), Some(ko_dist)) => {
                    ListConstruction::KOpt(ListKOptPhase::new(
                        k,
                        entity_count,
                        get_route,
                        set_route,
                        ko_depot,
                        ko_dist,
                        k_opt_feasible_fn,
                        descriptor_index,
                    ))
                }
                _ => {
                    panic!(
                        "list_k_opt requires k_opt_get_route, k_opt_set_route, \
                         k_opt_depot_fn, and k_opt_distance_fn"
                    );
                }
            }
        }
        _ => {
            // Default: ListCheapestInsertion
            ListConstruction::CheapestInsertion(ListCheapestInsertionPhase::new(
                element_count,
                get_assigned,
                entity_count,
                list_len,
                list_insert,
                list_remove,
                index_to_element,
                descriptor_index,
            ))
        }
    }
}

// Builds the local search phase from config or defaults.
pub fn build_list_local_search<S, V, DM, IDM>(
    config: &SolverConfig,
    ctx: &ListContext<S, V, DM, IDM>,
) -> ListLocalSearch<S, V, DM, IDM>
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + fmt::Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone,
{
    let ls_config = config.phases.iter().find_map(|p| {
        if let PhaseConfig::LocalSearch(ls) = p {
            Some(ls)
        } else {
            None
        }
    });

    let Some(ls) = ls_config else {
        // Default: LA(400) + AcceptedCount(4) + Union(NearbyChange(20), NearbySwap(20), Reverse)
        let acceptor = LateAcceptanceAcceptor::<S>::new(400);
        let forager = AcceptedCountForager::new(4);
        let move_selector = ListMoveSelectorBuilder::build(None, ctx);
        return ListLocalSearch::Default(LocalSearchPhase::new(
            move_selector,
            acceptor,
            forager,
            None,
        ));
    };

    let acceptor = ls
        .acceptor
        .as_ref()
        .map(|ac| AcceptorBuilder::build::<S>(ac))
        .unwrap_or_else(|| AnyAcceptor::LateAcceptance(LateAcceptanceAcceptor::<S>::new(400)));

    let forager = ForagerBuilder::build::<S>(ls.forager.as_ref());
    let move_selector = ListMoveSelectorBuilder::build(ls.move_selector.as_ref(), ctx);

    ListLocalSearch::Config(LocalSearchPhase::new(
        move_selector,
        acceptor,
        forager,
        None,
    ))
}
