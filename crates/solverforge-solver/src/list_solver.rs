//! List variable solver for routing and scheduling problems.
//!
//! This module provides `run_list_solver` for problems using list variables
//! (e.g., vehicle routes, shift schedules). The solver configuration
//! (construction type, move selectors, acceptor, forager, termination) is
//! driven by `solver.toml`.
//!
//! Logging levels:
//! - **INFO**: Solver start/end, phase summaries, problem scale
//! - **DEBUG**: Individual steps with timing and scores
//! - **TRACE**: Move evaluation details

use std::fmt;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use solverforge_config::{ConstructionHeuristicType, PhaseConfig, SolverConfig};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::{ConstraintSet, Director, ScoreDirector};
use tokio::sync::mpsc;
use tracing::info;

use crate::builder::list_selector::ListLeafSelector;
use crate::builder::{
    AcceptorBuilder, AnyAcceptor, AnyForager, ForagerBuilder, ListContext, ListMoveSelectorBuilder,
};
use crate::heuristic::r#move::ListMoveImpl;
use crate::heuristic::selector::decorator::VecUnionSelector;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::manager::{ListCheapestInsertionPhase, ListRegretInsertionPhase};
use crate::phase::localsearch::{AcceptedCountForager, LateAcceptanceAcceptor, LocalSearchPhase};
use crate::scope::BestSolutionCallback;
use crate::scope::SolverScope;
use crate::solver::Solver;
use crate::termination::{
    BestScoreTermination, OrTermination, StepCountTermination, Termination, TimeTermination,
    UnimprovedStepCountTermination, UnimprovedTimeTermination,
};

/// Default time limit in seconds for list solvers.
const DEFAULT_TIME_LIMIT_SECS: u64 = 60;

/// Monomorphized termination enum reused from basic solver.
pub(crate) enum AnyListTermination<S: PlanningSolution, D: Director<S>> {
    Default(OrTermination<(TimeTermination,), S, D>),
    WithBestScore(OrTermination<(TimeTermination, BestScoreTermination<S::Score>), S, D>),
    WithStepCount(OrTermination<(TimeTermination, StepCountTermination), S, D>),
    WithUnimprovedStep(OrTermination<(TimeTermination, UnimprovedStepCountTermination<S>), S, D>),
    WithUnimprovedTime(OrTermination<(TimeTermination, UnimprovedTimeTermination<S>), S, D>),
}

impl<S: PlanningSolution, D: Director<S>> fmt::Debug for AnyListTermination<S, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default(_) => write!(f, "AnyListTermination::Default"),
            Self::WithBestScore(_) => write!(f, "AnyListTermination::WithBestScore"),
            Self::WithStepCount(_) => write!(f, "AnyListTermination::WithStepCount"),
            Self::WithUnimprovedStep(_) => write!(f, "AnyListTermination::WithUnimprovedStep"),
            Self::WithUnimprovedTime(_) => write!(f, "AnyListTermination::WithUnimprovedTime"),
        }
    }
}

impl<S: PlanningSolution, D: Director<S>, BestCb: BestSolutionCallback<S>> Termination<S, D, BestCb>
    for AnyListTermination<S, D>
where
    S::Score: Score,
{
    fn is_terminated(&self, solver_scope: &SolverScope<S, D, BestCb>) -> bool {
        match self {
            Self::Default(t) => t.is_terminated(solver_scope),
            Self::WithBestScore(t) => t.is_terminated(solver_scope),
            Self::WithStepCount(t) => t.is_terminated(solver_scope),
            Self::WithUnimprovedStep(t) => t.is_terminated(solver_scope),
            Self::WithUnimprovedTime(t) => t.is_terminated(solver_scope),
        }
    }

    fn install_inphase_limits(&self, solver_scope: &mut SolverScope<S, D, BestCb>) {
        match self {
            Self::Default(t) => t.install_inphase_limits(solver_scope),
            Self::WithBestScore(t) => t.install_inphase_limits(solver_scope),
            Self::WithStepCount(t) => t.install_inphase_limits(solver_scope),
            Self::WithUnimprovedStep(t) => t.install_inphase_limits(solver_scope),
            Self::WithUnimprovedTime(t) => t.install_inphase_limits(solver_scope),
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

/// Monomorphized local search enum for list solver.
// Variants intentionally differ in size; this enum is constructed once per solve, not in hot paths.
#[allow(clippy::large_enum_variant)]
enum ListLocalSearch<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: CrossEntityDistanceMeter<S>,
    IDM: CrossEntityDistanceMeter<S>,
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
}

/// Solves a list variable problem using construction heuristic + local search.
///
/// Called by macro-generated `solve()` methods for solutions using
/// `#[shadow_variable_updates]` (list variables). When phases are configured in
/// `solver.toml`, the construction type, acceptor, forager, and move selectors
/// are built from config; otherwise defaults are used.
///
/// # Type Parameters
///
/// * `S` - The solution type
/// * `V` - The list element type
/// * `C` - The constraint set type
/// * `DM` - Cross-entity distance meter type
/// * `IDM` - Intra-entity distance meter type
///
/// # Default Behavior (no config)
///
/// - Construction: `ListCheapestInsertion`
/// - Acceptor: `LateAcceptance(400)`
/// - Forager: `AcceptedCount(4)`
/// - Move selector: `Union(NearbyListChange(20), NearbyListSwap(20), ListReverse)`
/// - Termination: 60s
#[allow(clippy::too_many_arguments)]
pub fn run_list_solver<S, V, C, DM, IDM>(
    mut solution: S,
    finalize_fn: fn(&mut S),
    constraints_fn: fn() -> C,
    // List operation function pointers
    list_len: fn(&S, usize) -> usize,
    list_remove: fn(&mut S, usize, usize) -> Option<V>,
    list_insert: fn(&mut S, usize, usize, V),
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_set: fn(&mut S, usize, usize, V),
    list_reverse: fn(&mut S, usize, usize, usize),
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    ruin_remove: fn(&mut S, usize, usize) -> V,
    ruin_insert: fn(&mut S, usize, usize, V),
    // Construction function pointers
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<V>,
    entity_count: fn(&S) -> usize,
    list_remove_for_construction: fn(&mut S, usize, usize) -> V,
    index_to_element: fn(&S, usize) -> V,
    // Distance meters
    cross_distance_meter: DM,
    intra_distance_meter: IDM,
    // Metadata
    variable_name: &'static str,
    descriptor_index: usize,
    // Solver infrastructure
    descriptor: fn() -> SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    terminate: Option<&AtomicBool>,
    sender: mpsc::UnboundedSender<(S, S::Score)>,
) -> S
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
    V: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + fmt::Debug + 'static,
    C: ConstraintSet<S, S::Score>,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone,
{
    finalize_fn(&mut solution);

    let config = SolverConfig::load("solver.toml").unwrap_or_default();
    let n_entities = entity_count(&solution);
    let n_elements = element_count(&solution);

    info!(
        event = "solve_start",
        entity_count = n_entities,
        element_count = n_elements,
    );

    let constraints = constraints_fn();
    let director = ScoreDirector::with_descriptor(
        solution,
        constraints,
        descriptor(),
        entity_count_by_descriptor,
    );

    if n_entities == 0 {
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();
        let score = solver_scope.calculate_score();
        info!(event = "solve_end", score = %score);
        let solution = solver_scope.take_best_or_working_solution();
        let _ = sender.send((solution.clone(), score));
        return solution;
    }

    // Build construction phase
    let construction = build_list_construction::<S, V>(
        &config,
        element_count,
        get_assigned,
        entity_count,
        list_len,
        list_insert,
        list_remove_for_construction,
        index_to_element,
        descriptor_index,
    );

    let ctx = ListContext::new(
        list_len,
        list_remove,
        list_insert,
        list_get,
        list_set,
        list_reverse,
        sublist_remove,
        sublist_insert,
        ruin_remove,
        ruin_insert,
        entity_count,
        cross_distance_meter,
        intra_distance_meter,
        variable_name,
        descriptor_index,
    );

    // Build local search phase
    let local_search = build_list_local_search::<S, V, DM, IDM>(&config, &ctx);

    // Build termination
    let term_config = config.termination.as_ref();
    let time_limit = term_config
        .and_then(|c| c.time_limit())
        .unwrap_or(Duration::from_secs(DEFAULT_TIME_LIMIT_SECS));
    let time = TimeTermination::new(time_limit);

    let best_score_target: Option<S::Score> = term_config
        .and_then(|c| c.best_score_limit.as_ref())
        .and_then(|s| S::Score::parse(s).ok());

    let termination: AnyListTermination<S, ScoreDirector<S, C>> =
        if let Some(target) = best_score_target {
            AnyListTermination::WithBestScore(OrTermination::new((
                time,
                BestScoreTermination::new(target),
            )))
        } else if let Some(step_limit) = term_config.and_then(|c| c.step_count_limit) {
            AnyListTermination::WithStepCount(OrTermination::new((
                time,
                StepCountTermination::new(step_limit),
            )))
        } else if let Some(unimproved_step_limit) =
            term_config.and_then(|c| c.unimproved_step_count_limit)
        {
            AnyListTermination::WithUnimprovedStep(OrTermination::new((
                time,
                UnimprovedStepCountTermination::<S>::new(unimproved_step_limit),
            )))
        } else if let Some(unimproved_time) = term_config.and_then(|c| c.unimproved_time_limit()) {
            AnyListTermination::WithUnimprovedTime(OrTermination::new((
                time,
                UnimprovedTimeTermination::<S>::new(unimproved_time),
            )))
        } else {
            AnyListTermination::Default(OrTermination::new((time,)))
        };

    let callback_sender = sender.clone();
    let callback = move |solution: &S| {
        let score = solution.score().unwrap_or_default();
        let _ = callback_sender.send((solution.clone(), score));
    };

    // Run solver using the construction + local search phases
    let result = match (construction, local_search) {
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
    };

    let final_score = result.solution.score().unwrap_or_default();
    let _ = sender.send((result.solution.clone(), final_score));

    info!(
        event = "solve_end",
        score = %final_score,
        steps = result.stats.step_count,
        moves_evaluated = result.stats.moves_evaluated,
    );
    result.solution
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
