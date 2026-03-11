//! List variable solver for routing and scheduling problems.
//!
//! This module provides `ListSpec` for problems with list variables
//! (e.g., vehicle routes, shift schedules). The solver configuration
//! (construction type, move selectors, acceptor, forager, termination) is
//! driven by `solver.toml`.
//!
//! Logging levels:
//! - **INFO**: Solver start/end, phase summaries, problem scale
//! - **DEBUG**: Individual steps with timing and scores
//! - **TRACE**: Move evaluation details

use std::fmt;
use std::marker::PhantomData;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use solverforge_config::{ConstructionHeuristicType, PhaseConfig, SolverConfig};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::{ConstraintSet, ScoreDirector};
use tracing::info;

use crate::builder::list_selector::ListLeafSelector;
use crate::builder::{
    AcceptorBuilder, AnyAcceptor, AnyForager, ForagerBuilder, ListContext, ListMoveSelectorBuilder,
};
use crate::heuristic::r#move::ListMoveImpl;
use crate::heuristic::selector::decorator::VecUnionSelector;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::manager::{ListCheapestInsertionPhase, ListClarkeWrightPhase, ListRegretInsertionPhase};
use crate::phase::localsearch::{AcceptedCountForager, LateAcceptanceAcceptor, LocalSearchPhase};
use crate::problem_spec::ProblemSpec;
use crate::run::AnyTermination;
use crate::solver::{SolveResult, Solver};

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

/// Monomorphized local search enum for list solver.
// Variants intentionally differ in size; this enum is constructed once per solve, not in hot paths.
#[allow(clippy::large_enum_variant)]
enum ListLocalSearch<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: CrossEntityDistanceMeter<S>,
    IDM: CrossEntityDistanceMeter<S> + 'static,
{
    Default(DefaultListLocalSearch<S, V, DM, IDM>),
    Config(ConfigListLocalSearch<S, V, DM, IDM>),
}

/// Construction phase enum for list solver.
enum ListConstruction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
{
    CheapestInsertion(ListCheapestInsertionPhase<S, V>),
    RegretInsertion(ListRegretInsertionPhase<S, V>),
    ClarkeWright(ListClarkeWrightPhase<S, V>),
}

/// Problem specification for list variable problems.
///
/// Passed to `run_solver` to provide problem-specific construction and local
/// search phases for solutions using `#[shadow_variable_updates]` (list variables).
pub struct ListSpec<S, V, DM, IDM> {
    // List operation function pointers
    pub list_len: fn(&S, usize) -> usize,
    pub list_remove: fn(&mut S, usize, usize) -> Option<V>,
    pub list_insert: fn(&mut S, usize, usize, V),
    pub list_get: fn(&S, usize, usize) -> Option<V>,
    pub list_set: fn(&mut S, usize, usize, V),
    pub list_reverse: fn(&mut S, usize, usize, usize),
    pub sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    pub sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    pub ruin_remove: fn(&mut S, usize, usize) -> V,
    pub ruin_insert: fn(&mut S, usize, usize, V),
    // Construction function pointers
    pub element_count: fn(&S) -> usize,
    pub get_assigned: fn(&S) -> Vec<V>,
    pub entity_count: fn(&S) -> usize,
    pub list_remove_for_construction: fn(&mut S, usize, usize) -> V,
    pub index_to_element: fn(&S, usize) -> V,
    // Distance meters
    pub cross_distance_meter: DM,
    pub intra_distance_meter: IDM,
    // Clarke-Wright fields (all None if not using Clarke-Wright construction)
    pub depot_fn: Option<fn(&S) -> usize>,
    pub distance_fn: Option<fn(usize, usize) -> i64>,
    pub element_load_fn: Option<fn(&S, usize) -> i64>,
    pub capacity_fn: Option<fn(&S) -> i64>,
    pub assign_route_fn: Option<fn(&mut S, usize, Vec<V>)>,
    // Metadata
    pub variable_name: &'static str,
    pub descriptor_index: usize,
    pub _phantom: PhantomData<fn() -> S>,
}

impl<S, V, C, DM, IDM> ProblemSpec<S, C> for ListSpec<S, V, DM, IDM>
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
    V: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + fmt::Debug + 'static,
    C: ConstraintSet<S, S::Score>,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    fn is_trivial(&self, solution: &S) -> bool {
        (self.entity_count)(solution) == 0
    }

    fn default_time_limit_secs(&self) -> u64 {
        60
    }

    fn log_scale(&self, solution: &S) {
        info!(
            event = "solve_start",
            entity_count = (self.entity_count)(solution),
            element_count = (self.element_count)(solution),
        );
    }

    fn build_and_solve(
        self,
        director: ScoreDirector<S, C>,
        config: &SolverConfig,
        time_limit: Duration,
        termination: AnyTermination<S, ScoreDirector<S, C>>,
        terminate: Option<&AtomicBool>,
        callback: impl Fn(&S) + Send + Sync,
    ) -> SolveResult<S> {
        let construction = build_list_construction::<S, V>(
            config,
            self.element_count,
            self.get_assigned,
            self.entity_count,
            self.list_len,
            self.list_insert,
            self.list_remove_for_construction,
            self.index_to_element,
            self.descriptor_index,
            self.depot_fn,
            self.distance_fn,
            self.element_load_fn,
            self.capacity_fn,
            self.assign_route_fn,
        );

        let ctx = ListContext::new(
            self.list_len,
            self.list_remove,
            self.list_insert,
            self.list_get,
            self.list_set,
            self.list_reverse,
            self.sublist_remove,
            self.sublist_insert,
            self.ruin_remove,
            self.ruin_insert,
            self.entity_count,
            self.cross_distance_meter,
            self.intra_distance_meter,
            self.variable_name,
            self.descriptor_index,
        );

        let local_search = build_list_local_search::<S, V, DM, IDM>(config, &ctx);

        match (construction, local_search) {
            (ListConstruction::CheapestInsertion(c), ListLocalSearch::Default(ls)) => {
                let solver = Solver::new(((), c, ls))
                    .with_termination(termination)
                    .with_time_limit(time_limit)
                    .with_best_solution_callback(callback);
                if let Some(flag) = terminate {
                    solver.with_terminate(flag).solve(director)
                } else {
                    solver.solve(director)
                }
            }
            (ListConstruction::CheapestInsertion(c), ListLocalSearch::Config(ls)) => {
                let solver = Solver::new(((), c, ls))
                    .with_termination(termination)
                    .with_time_limit(time_limit)
                    .with_best_solution_callback(callback);
                if let Some(flag) = terminate {
                    solver.with_terminate(flag).solve(director)
                } else {
                    solver.solve(director)
                }
            }
            (ListConstruction::RegretInsertion(c), ListLocalSearch::Default(ls)) => {
                let solver = Solver::new(((), c, ls))
                    .with_termination(termination)
                    .with_time_limit(time_limit)
                    .with_best_solution_callback(callback);
                if let Some(flag) = terminate {
                    solver.with_terminate(flag).solve(director)
                } else {
                    solver.solve(director)
                }
            }
            (ListConstruction::RegretInsertion(c), ListLocalSearch::Config(ls)) => {
                let solver = Solver::new(((), c, ls))
                    .with_termination(termination)
                    .with_time_limit(time_limit)
                    .with_best_solution_callback(callback);
                if let Some(flag) = terminate {
                    solver.with_terminate(flag).solve(director)
                } else {
                    solver.solve(director)
                }
            }
            (ListConstruction::ClarkeWright(c), ListLocalSearch::Default(ls)) => {
                let solver = Solver::new(((), c, ls))
                    .with_termination(termination)
                    .with_time_limit(time_limit)
                    .with_best_solution_callback(callback);
                if let Some(flag) = terminate {
                    solver.with_terminate(flag).solve(director)
                } else {
                    solver.solve(director)
                }
            }
            (ListConstruction::ClarkeWright(c), ListLocalSearch::Config(ls)) => {
                let solver = Solver::new(((), c, ls))
                    .with_termination(termination)
                    .with_time_limit(time_limit)
                    .with_best_solution_callback(callback);
                if let Some(flag) = terminate {
                    solver.with_terminate(flag).solve(director)
                } else {
                    solver.solve(director)
                }
            }
        }
    }
}

/// Builds the construction phase from config or defaults to cheapest insertion.
#[allow(clippy::too_many_arguments)]
fn build_list_construction<S, V>(
    config: &SolverConfig,
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<V>,
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_insert: fn(&mut S, usize, usize, V),
    list_remove: fn(&mut S, usize, usize) -> V,
    index_to_element: fn(&S, usize) -> V,
    descriptor_index: usize,
    depot_fn: Option<fn(&S) -> usize>,
    distance_fn: Option<fn(usize, usize) -> i64>,
    element_load_fn: Option<fn(&S, usize) -> i64>,
    capacity_fn: Option<fn(&S) -> i64>,
    assign_route_fn: Option<fn(&mut S, usize, Vec<V>)>,
) -> ListConstruction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + fmt::Debug + 'static,
{
    let ch_type = config
        .phases
        .iter()
        .find_map(|p| {
            if let PhaseConfig::ConstructionHeuristic(ch) = p {
                Some(ch.construction_heuristic_type)
            } else {
                None
            }
        })
        .unwrap_or(ConstructionHeuristicType::ListCheapestInsertion);

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
                        descriptor_index,
                    ))
                }
                _ => {
                    tracing::warn!(
                        "ListClarkeWright selected but one or more required fields \
                         (depot_fn, distance_fn, element_load_fn, capacity_fn, assign_route_fn) \
                         are None — falling back to ListCheapestInsertion"
                    );
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

/// Builds the local search phase from config or defaults.
fn build_list_local_search<S, V, DM, IDM>(
    config: &SolverConfig,
    ctx: &ListContext<S, V, DM, IDM>,
) -> ListLocalSearch<S, V, DM, IDM>
where
    S: PlanningSolution,
    S::Score: Score,
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
