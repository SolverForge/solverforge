use std::fmt::{self, Debug};
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, PhaseConfig, SolverConfig,
};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};

use crate::builder::{build_local_search, build_vnd, LocalSearch, ModelContext, Vnd};
use crate::descriptor_standard::{
    build_descriptor_construction, standard_target_matches, standard_work_remaining_with_frontier,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::manager::{
    ListCheapestInsertionPhase, ListClarkeWrightPhase, ListKOptPhase, ListRegretInsertionPhase,
};
use crate::phase::{sequence::PhaseSequence, Phase};
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepScope};

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "list_solver_tests.rs"]
mod list_tests;

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

pub trait ListVariableEntity<S> {
    type CrossDistanceMeter: CrossEntityDistanceMeter<S> + Clone + fmt::Debug;
    type IntraDistanceMeter: CrossEntityDistanceMeter<S> + Clone + fmt::Debug + 'static;

    const HAS_LIST_VARIABLE: bool;
    const LIST_VARIABLE_NAME: &'static str;
    const LIST_ELEMENT_SOURCE: Option<&'static str>;

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

enum ListConstruction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + 'static,
{
    RoundRobin(ListRoundRobinPhase<S, V>),
    CheapestInsertion(ListCheapestInsertionPhase<S, V>),
    RegretInsertion(ListRegretInsertionPhase<S, V>),
    ClarkeWright(ListClarkeWrightPhase<S, V>),
    KOpt(ListKOptPhase<S, V>),
}

impl<S, V> Debug for ListConstruction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
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

impl<S, V, D, BestCb> Phase<S, D, BestCb> for ListConstruction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    D: solverforge_scoring::Director<S>,
    BestCb: crate::scope::ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, BestCb>) {
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

struct ListRoundRobinPhase<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Send + Sync + 'static,
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

impl<S, V> Debug for ListRoundRobinPhase<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListRoundRobinPhase").finish()
    }
}

impl<S, V, D, ProgressCb> Phase<S, D, ProgressCb> for ListRoundRobinPhase<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Send + Sync + Debug + 'static,
    D: solverforge_scoring::Director<S>,
    ProgressCb: ProgressCallback<S>,
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
            if phase_scope
                .solver_scope_mut()
                .should_terminate_construction()
            {
                break;
            }

            let element =
                (self.index_to_element)(phase_scope.score_director().working_solution(), elem_idx);
            if assigned_set.contains(&element) {
                continue;
            }

            let mut step_scope = StepScope::new(&mut phase_scope);

            step_scope.apply_committed_change(|sd| {
                let insert_pos = (self.list_len)(sd.working_solution(), entity_idx);
                sd.before_variable_changed(self.descriptor_index, entity_idx);
                (self.list_insert)(sd.working_solution_mut(), entity_idx, insert_pos, element);
                sd.after_variable_changed(self.descriptor_index, entity_idx);
            });

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

pub struct ConstructionArgs<S, V> {
    pub element_count: fn(&S) -> usize,
    pub assigned_elements: fn(&S) -> Vec<V>,
    pub entity_count: fn(&S) -> usize,
    pub list_len: fn(&S, usize) -> usize,
    pub list_insert: fn(&mut S, usize, usize, V),
    pub list_remove: fn(&mut S, usize, usize) -> V,
    pub index_to_element: fn(&S, usize) -> V,
    pub descriptor_index: usize,
    pub entity_type_name: &'static str,
    pub variable_name: &'static str,
    pub depot_fn: Option<fn(&S) -> usize>,
    pub distance_fn: Option<fn(&S, usize, usize) -> i64>,
    pub element_load_fn: Option<fn(&S, usize) -> i64>,
    pub capacity_fn: Option<fn(&S) -> i64>,
    pub assign_route_fn: Option<fn(&mut S, usize, Vec<V>)>,
    pub merge_feasible_fn: Option<fn(&S, &[usize]) -> bool>,
    pub k_opt_get_route: Option<fn(&S, usize) -> Vec<usize>>,
    pub k_opt_set_route: Option<fn(&mut S, usize, Vec<usize>)>,
    pub k_opt_depot_fn: Option<fn(&S, usize) -> usize>,
    pub k_opt_distance_fn: Option<fn(&S, usize, usize) -> i64>,
    pub k_opt_feasible_fn: Option<fn(&S, usize, &[usize]) -> bool>,
}

impl<S, V> Clone for ConstructionArgs<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> Copy for ConstructionArgs<S, V> {}

fn list_work_remaining<S, V>(args: &ConstructionArgs<S, V>, solution: &S) -> bool
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Send + Sync + 'static,
{
    (args.assigned_elements)(solution).len() < (args.element_count)(solution)
}

fn has_explicit_target(config: &ConstructionHeuristicConfig) -> bool {
    config.target.variable_name.is_some() || config.target.entity_class.is_some()
}

fn is_list_only_heuristic(heuristic: ConstructionHeuristicType) -> bool {
    matches!(
        heuristic,
        ConstructionHeuristicType::ListRoundRobin
            | ConstructionHeuristicType::ListCheapestInsertion
            | ConstructionHeuristicType::ListRegretInsertion
            | ConstructionHeuristicType::ListClarkeWright
            | ConstructionHeuristicType::ListKOpt
    )
}

fn is_standard_only_heuristic(heuristic: ConstructionHeuristicType) -> bool {
    matches!(
        heuristic,
        ConstructionHeuristicType::FirstFitDecreasing
            | ConstructionHeuristicType::WeakestFit
            | ConstructionHeuristicType::WeakestFitDecreasing
            | ConstructionHeuristicType::StrongestFit
            | ConstructionHeuristicType::StrongestFitDecreasing
            | ConstructionHeuristicType::AllocateEntityFromQueue
            | ConstructionHeuristicType::AllocateToValueFromQueue
    )
}

fn list_target_matches<S, V>(
    config: &ConstructionHeuristicConfig,
    list_construction: &ConstructionArgs<S, V>,
) -> bool
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Send + Sync + 'static,
{
    if !has_explicit_target(config) {
        return false;
    }

    config
        .target
        .variable_name
        .as_deref()
        .is_none_or(|name| name == list_construction.variable_name)
        && config
            .target
            .entity_class
            .as_deref()
            .is_none_or(|name| name == list_construction.entity_type_name)
}

fn matching_list_construction<S, V>(
    config: Option<&ConstructionHeuristicConfig>,
    list_construction: &[ConstructionArgs<S, V>],
) -> Vec<ConstructionArgs<S, V>>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Send + Sync + 'static,
{
    let Some(config) = config else {
        return list_construction.to_vec();
    };

    if !has_explicit_target(config) {
        return list_construction.to_vec();
    }

    list_construction
        .iter()
        .copied()
        .filter(|args| list_target_matches(config, args))
        .collect()
}

fn normalize_list_construction_config(
    config: Option<&ConstructionHeuristicConfig>,
) -> Option<ConstructionHeuristicConfig> {
    let mut config = config.cloned()?;
    config.construction_heuristic_type = match config.construction_heuristic_type {
        ConstructionHeuristicType::FirstFit | ConstructionHeuristicType::CheapestInsertion => {
            ConstructionHeuristicType::ListCheapestInsertion
        }
        other => other,
    };
    Some(config)
}

fn build_list_construction<S, V>(
    config: Option<&ConstructionHeuristicConfig>,
    args: &ConstructionArgs<S, V>,
) -> ListConstruction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
{
    let Some((ch_type, k)) = config.map(|cfg| (cfg.construction_heuristic_type, cfg.k)) else {
        return ListConstruction::CheapestInsertion(ListCheapestInsertionPhase::new(
            args.element_count,
            args.assigned_elements,
            args.entity_count,
            args.list_len,
            args.list_insert,
            args.list_remove,
            args.index_to_element,
            args.descriptor_index,
        ));
    };

    match ch_type {
        ConstructionHeuristicType::ListRoundRobin => {
            ListConstruction::RoundRobin(ListRoundRobinPhase {
                element_count: args.element_count,
                get_assigned: args.assigned_elements,
                entity_count: args.entity_count,
                list_len: args.list_len,
                list_insert: args.list_insert,
                index_to_element: args.index_to_element,
                descriptor_index: args.descriptor_index,
                _phantom: PhantomData,
            })
        }
        ConstructionHeuristicType::ListRegretInsertion => {
            ListConstruction::RegretInsertion(ListRegretInsertionPhase::new(
                args.element_count,
                args.assigned_elements,
                args.entity_count,
                args.list_len,
                args.list_insert,
                args.list_remove,
                args.index_to_element,
                args.descriptor_index,
            ))
        }
        ConstructionHeuristicType::ListClarkeWright => {
            match (
                args.depot_fn,
                args.distance_fn,
                args.element_load_fn,
                args.capacity_fn,
                args.assign_route_fn,
            ) {
                (Some(depot), Some(dist), Some(load), Some(cap), Some(assign)) => {
                    ListConstruction::ClarkeWright(ListClarkeWrightPhase::new(
                        args.element_count,
                        args.assigned_elements,
                        args.entity_count,
                        args.list_len,
                        assign,
                        args.index_to_element,
                        depot,
                        dist,
                        load,
                        cap,
                        args.merge_feasible_fn,
                        args.descriptor_index,
                    ))
                }
                _ => {
                    panic!(
                        "list_clarke_wright requires depot_fn, distance_fn, element_load_fn, capacity_fn, and assign_route_fn"
                    );
                }
            }
        }
        ConstructionHeuristicType::ListKOpt => match (
            args.k_opt_get_route,
            args.k_opt_set_route,
            args.k_opt_depot_fn,
            args.k_opt_distance_fn,
        ) {
            (Some(get_route), Some(set_route), Some(ko_depot), Some(ko_dist)) => {
                ListConstruction::KOpt(ListKOptPhase::new(
                    k,
                    args.entity_count,
                    get_route,
                    set_route,
                    ko_depot,
                    ko_dist,
                    args.k_opt_feasible_fn,
                    args.descriptor_index,
                ))
            }
            _ => {
                panic!(
                    "list_k_opt requires k_opt_get_route, k_opt_set_route, k_opt_depot_fn, and k_opt_distance_fn"
                );
            }
        },
        ConstructionHeuristicType::ListCheapestInsertion => {
            ListConstruction::CheapestInsertion(ListCheapestInsertionPhase::new(
                args.element_count,
                args.assigned_elements,
                args.entity_count,
                args.list_len,
                args.list_insert,
                args.list_remove,
                args.index_to_element,
                args.descriptor_index,
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

pub struct Construction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + 'static,
{
    config: Option<ConstructionHeuristicConfig>,
    descriptor: SolutionDescriptor,
    list_construction: Vec<ConstructionArgs<S, V>>,
}

impl<S, V> Construction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + 'static,
{
    fn new(
        config: Option<ConstructionHeuristicConfig>,
        descriptor: SolutionDescriptor,
        list_construction: Vec<ConstructionArgs<S, V>>,
    ) -> Self {
        Self {
            config,
            descriptor,
            list_construction,
        }
    }
}

impl<S, V> Debug for Construction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Construction")
            .field("config", &self.config)
            .field("list_construction_count", &self.list_construction.len())
            .finish()
    }
}

impl<S, V, D, ProgressCb> Phase<S, D, ProgressCb> for Construction<S, V>
where
    S: PlanningSolution + 'static,
    V: Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    D: solverforge_scoring::Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        let config = self.config.as_ref();
        let explicit_target = config.is_some_and(has_explicit_target);
        let entity_class = config.and_then(|cfg| cfg.target.entity_class.as_deref());
        let variable_name = config.and_then(|cfg| cfg.target.variable_name.as_deref());
        let standard_matches = config.is_some_and(|_| {
            standard_target_matches(&self.descriptor, entity_class, variable_name)
        });
        let matching_list_construction =
            matching_list_construction(config, &self.list_construction);
        let standard_remaining = standard_work_remaining_with_frontier(
            &self.descriptor,
            solver_scope.standard_construction_frontier(),
            solver_scope.solution_revision(),
            if explicit_target { entity_class } else { None },
            if explicit_target { variable_name } else { None },
            solver_scope.working_solution(),
        );
        let mut ran_child_phase = false;

        if let Some(cfg) = config {
            if explicit_target && !standard_matches && matching_list_construction.is_empty() {
                panic!(
                    "construction heuristic matched no planning variables for entity_class={:?} variable_name={:?}",
                    cfg.target.entity_class,
                    cfg.target.variable_name
                );
            }

            let heuristic = cfg.construction_heuristic_type;
            if is_list_only_heuristic(heuristic) {
                assert!(
                    !self.list_construction.is_empty(),
                    "list construction heuristic {:?} configured against a solution with no planning list variable",
                    heuristic
                );
                assert!(
                    !explicit_target || !matching_list_construction.is_empty(),
                    "list construction heuristic {:?} does not match the targeted planning list variable for entity_class={:?} variable_name={:?}",
                    heuristic,
                    cfg.target.entity_class,
                    cfg.target.variable_name
                );
                ran_child_phase = self.solve_list(solver_scope, &matching_list_construction);
                if !ran_child_phase {
                    finalize_noop_construction(solver_scope);
                }
                return;
            }

            if is_standard_only_heuristic(heuristic) {
                assert!(
                    !explicit_target || standard_matches,
                    "standard construction heuristic {:?} does not match targeted standard planning variables for entity_class={:?} variable_name={:?}",
                    heuristic,
                    cfg.target.entity_class,
                    cfg.target.variable_name
                );
                if standard_remaining {
                    build_descriptor_construction(Some(cfg), &self.descriptor).solve(solver_scope);
                    ran_child_phase = true;
                }
                if !ran_child_phase {
                    finalize_noop_construction(solver_scope);
                }
                return;
            }
        }

        if self.list_construction.is_empty() {
            if standard_remaining {
                build_descriptor_construction(config, &self.descriptor).solve(solver_scope);
                ran_child_phase = true;
            }
            if !ran_child_phase {
                finalize_noop_construction(solver_scope);
            }
            return;
        }

        let list_remaining = matching_list_construction
            .iter()
            .any(|args| list_work_remaining(args, solver_scope.working_solution()));

        if standard_remaining {
            build_descriptor_construction(config, &self.descriptor).solve(solver_scope);
            ran_child_phase = true;
        }
        if list_remaining {
            ran_child_phase |= self.solve_list(solver_scope, &matching_list_construction);
        }
        if !ran_child_phase {
            finalize_noop_construction(solver_scope);
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "Construction"
    }
}

impl<S, V> Construction<S, V>
where
    S: PlanningSolution + 'static,
    V: Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
{
    fn solve_list<D, ProgressCb>(
        &self,
        solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
        list_construction: &[ConstructionArgs<S, V>],
    ) -> bool
    where
        D: solverforge_scoring::Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        if list_construction.is_empty() {
            panic!("list construction configured against a scalar-only context");
        }
        let normalized = normalize_list_construction_config(self.config.as_ref());
        let mut ran_phase = false;
        for args in list_construction {
            if !list_work_remaining(args, solver_scope.working_solution()) {
                continue;
            }
            build_list_construction(normalized.as_ref(), args).solve(solver_scope);
            ran_phase = true;
        }
        ran_phase
    }
}

fn finalize_noop_construction<S, D, ProgressCb>(
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) where
    S: PlanningSolution,
    D: solverforge_scoring::Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let had_best = solver_scope.best_score().is_some();
    solver_scope.update_best_solution();
    if had_best {
        solver_scope.promote_current_solution_on_score_tie();
    }
}

pub enum RuntimePhase<C, LS, VND> {
    Construction(C),
    LocalSearch(LS),
    Vnd(VND),
}

impl<C, LS, VND> Debug for RuntimePhase<C, LS, VND>
where
    C: Debug,
    LS: Debug,
    VND: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Construction(phase) => write!(f, "RuntimePhase::Construction({phase:?})"),
            Self::LocalSearch(phase) => write!(f, "RuntimePhase::LocalSearch({phase:?})"),
            Self::Vnd(phase) => write!(f, "RuntimePhase::Vnd({phase:?})"),
        }
    }
}

impl<S, D, ProgressCb, C, LS, VND> Phase<S, D, ProgressCb> for RuntimePhase<C, LS, VND>
where
    S: PlanningSolution,
    D: solverforge_scoring::Director<S>,
    ProgressCb: ProgressCallback<S>,
    C: Phase<S, D, ProgressCb> + Debug,
    LS: Phase<S, D, ProgressCb> + Debug,
    VND: Phase<S, D, ProgressCb> + Debug,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        match self {
            Self::Construction(phase) => phase.solve(solver_scope),
            Self::LocalSearch(phase) => phase.solve(solver_scope),
            Self::Vnd(phase) => phase.solve(solver_scope),
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "RuntimePhase"
    }
}

pub fn build_phases<S, V, DM, IDM>(
    config: &SolverConfig,
    descriptor: &SolutionDescriptor,
    model: &ModelContext<S, V, DM, IDM>,
    list_construction: Vec<ConstructionArgs<S, V>>,
) -> PhaseSequence<RuntimePhase<Construction<S, V>, LocalSearch<S, V, DM, IDM>, Vnd<S, V, DM, IDM>>>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    let mut phases = Vec::new();

    if config.phases.is_empty() {
        phases.push(RuntimePhase::Construction(Construction::new(
            None,
            descriptor.clone(),
            list_construction.clone(),
        )));
        phases.push(RuntimePhase::LocalSearch(build_local_search(
            None,
            model,
            config.random_seed,
        )));
        return PhaseSequence::new(phases);
    }

    for phase in &config.phases {
        match phase {
            PhaseConfig::ConstructionHeuristic(ch) => {
                phases.push(RuntimePhase::Construction(Construction::new(
                    Some(ch.clone()),
                    descriptor.clone(),
                    list_construction.clone(),
                )));
            }
            PhaseConfig::LocalSearch(ls) => {
                phases.push(RuntimePhase::LocalSearch(build_local_search(
                    Some(ls),
                    model,
                    config.random_seed,
                )));
            }
            PhaseConfig::Vnd(vnd) => {
                phases.push(RuntimePhase::Vnd(build_vnd(vnd, model, config.random_seed)));
            }
            _ => {
                panic!("unsupported phase in the runtime");
            }
        }
    }

    PhaseSequence::new(phases)
}
