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

use solverforge_config::{ConstructionHeuristicConfig, ConstructionHeuristicType};
use solverforge_core::domain::PlanningSolution;
use std::fmt;
use std::marker::PhantomData;

use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::manager::{
    ListCheapestInsertionPhase, ListClarkeWrightPhase, ListKOptPhase, ListRegretInsertionPhase,
};
use crate::scope::{PhaseScope, SolverScope, StepScope};

/// Hidden list metadata emitted by `#[planning_entity]` and consumed by
/// macro-generated solve code.
pub struct ListVariableMetadata<S, DM, IDM> {
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
pub trait ListVariableEntity<S> {
    type CrossDistanceMeter: CrossEntityDistanceMeter<S> + Clone + fmt::Debug;
    type IntraDistanceMeter: CrossEntityDistanceMeter<S> + Clone + fmt::Debug + 'static;

    const HAS_STOCK_LIST_VARIABLE: bool;
    const STOCK_LIST_VARIABLE_NAME: &'static str;
    const STOCK_LIST_ELEMENT_SOURCE: Option<&'static str>;

    fn list_field(entity: &Self) -> &[usize];
    fn list_field_mut(entity: &mut Self) -> &mut Vec<usize>;
    fn list_metadata() -> ListVariableMetadata<S, Self::CrossDistanceMeter, Self::IntraDistanceMeter>;
}

impl<S, DM, IDM> ListVariableMetadata<S, DM, IDM> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
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

// Construction phase enum for list solver.
pub enum ListConstruction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
{
    RoundRobin(ListRoundRobinPhase<S, V>),
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
            Self::RoundRobin(phase) => write!(f, "ListConstruction::RoundRobin({phase:?})"),
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

impl<S, V, D, ProgressCb> crate::phase::Phase<S, D, ProgressCb> for ListConstruction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + fmt::Debug + 'static,
    D: solverforge_scoring::Director<S>,
    ProgressCb: crate::scope::ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut crate::scope::SolverScope<'_, S, D, ProgressCb>) {
        match self {
            Self::RoundRobin(phase) => phase.solve(solver_scope),
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

pub struct ListRoundRobinPhase<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
{
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<V>,
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_insert: fn(&mut S, usize, usize, V),
    index_to_element: fn(&S, usize) -> V,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V> fmt::Debug for ListRoundRobinPhase<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListRoundRobinPhase").finish()
    }
}

impl<S, V, D, ProgressCb> crate::phase::Phase<S, D, ProgressCb> for ListRoundRobinPhase<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + fmt::Debug + 'static,
    D: solverforge_scoring::Director<S>,
    ProgressCb: crate::scope::ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        let solution = phase_scope.score_director().working_solution();
        let n_elements = (self.element_count)(solution);
        let n_entities = (self.entity_count)(solution);

        if n_entities == 0 || n_elements == 0 {
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned: Vec<V> = (self.get_assigned)(phase_scope.score_director().working_solution());
        if assigned.len() >= n_elements {
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned_set: std::collections::HashSet<V> = assigned.into_iter().collect();
        let mut entity_idx = 0;

        for elem_idx in 0..n_elements {
            if phase_scope.solver_scope().should_terminate_construction() {
                break;
            }

            let element =
                (self.index_to_element)(phase_scope.score_director().working_solution(), elem_idx);
            if assigned_set.contains(&element) {
                continue;
            }

            let mut step_scope = StepScope::new(&mut phase_scope);

            {
                let sd = step_scope.score_director_mut();
                let insert_pos = (self.list_len)(sd.working_solution(), entity_idx);
                sd.before_variable_changed(self.descriptor_index, entity_idx);
                (self.list_insert)(sd.working_solution_mut(), entity_idx, insert_pos, element);
                sd.after_variable_changed(self.descriptor_index, entity_idx);
            }

            let step_score = step_scope.calculate_score();
            step_scope.set_step_score(step_score);
            step_scope.complete();

            entity_idx = (entity_idx + 1) % n_entities;
        }

        phase_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "ListRoundRobin"
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
    let Some((ch_type, k)) = config.map(|cfg| (cfg.construction_heuristic_type, cfg.k)) else {
        return ListConstruction::CheapestInsertion(ListCheapestInsertionPhase::new(
            element_count,
            get_assigned,
            entity_count,
            list_len,
            list_insert,
            list_remove,
            index_to_element,
            descriptor_index,
        ));
    };

    match ch_type {
        ConstructionHeuristicType::ListRoundRobin => {
            ListConstruction::RoundRobin(ListRoundRobinPhase {
                element_count,
                get_assigned,
                entity_count,
                list_len,
                list_insert,
                index_to_element,
                descriptor_index,
                _phantom: PhantomData,
            })
        }
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
        ConstructionHeuristicType::ListCheapestInsertion => {
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
        ConstructionHeuristicType::FirstFit | ConstructionHeuristicType::CheapestInsertion => {
            panic!(
                "generic construction heuristic {:?} must be normalized before list construction",
                ch_type
            );
        }
        ConstructionHeuristicType::FirstFitDecreasing
        | ConstructionHeuristicType::WeakestFit
        | ConstructionHeuristicType::WeakestFitDecreasing
        | ConstructionHeuristicType::StrongestFit
        | ConstructionHeuristicType::StrongestFitDecreasing
        | ConstructionHeuristicType::AllocateEntityFromQueue
        | ConstructionHeuristicType::AllocateToValueFromQueue => {
            panic!(
                "standard construction heuristic {:?} configured against a list variable",
                ch_type
            );
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

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_config::VariableTargetConfig;
    use solverforge_core::score::SoftScore;

    #[derive(Clone, Debug)]
    struct TestSolution {
        score: Option<SoftScore>,
    }

    impl PlanningSolution for TestSolution {
        type Score = SoftScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn config(kind: ConstructionHeuristicType) -> ConstructionHeuristicConfig {
        ConstructionHeuristicConfig {
            construction_heuristic_type: kind,
            target: VariableTargetConfig::default(),
            k: 2,
            termination: None,
        }
    }

    #[test]
    fn list_builder_rejects_unnormalized_generic_construction() {
        let panic = std::panic::catch_unwind(|| {
            let _ = build_list_construction::<TestSolution, usize>(
                Some(&config(ConstructionHeuristicType::FirstFit)),
                |_| 0,
                |_| Vec::new(),
                |_| 0,
                |_, _| 0,
                |_, _, _, _| {},
                |_, _, _| 0,
                |_, _| 0,
                0,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            );
        })
        .expect_err("unnormalized generic construction should panic");

        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&'static str>().copied())
            .unwrap_or("");
        assert!(message.contains("must be normalized before list construction"));
    }
}
