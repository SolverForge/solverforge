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
    build_descriptor_construction, standard_work_remaining_with_frontier,
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

struct ListRoundRobinPhase<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Send + Sync + 'static,
{
    ctx: crate::builder::ListVariableContext<S, V, DM, IDM>,
}

impl<S, V, DM, IDM> Debug for ListRoundRobinPhase<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListRoundRobinPhase").finish()
    }
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

fn list_work_remaining<S, V, DM, IDM>(
    ctx: &crate::builder::ListVariableContext<S, V, DM, IDM>,
    solution: &S,
) -> bool
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Send + Sync + 'static,
{
    (ctx.assigned_elements)(solution).len() < (ctx.element_count)(solution)
}

fn matching_list_variables<S, V, DM, IDM>(
    config: Option<&ConstructionHeuristicConfig>,
    model: &ModelContext<S, V, DM, IDM>,
) -> Vec<crate::builder::ListVariableContext<S, V, DM, IDM>>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Send + Sync + 'static,
    DM: Clone,
    IDM: Clone,
{
    let entity_class = config.and_then(|cfg| cfg.target.entity_class.as_deref());
    let variable_name = config.and_then(|cfg| cfg.target.variable_name.as_deref());
    let explicit_target = config.is_some_and(has_explicit_target);

    model
        .list_variables()
        .filter(|ctx| !explicit_target || ctx.matches_target(entity_class, variable_name))
        .cloned()
        .collect()
}

fn has_matching_scalar_target<S, V, DM, IDM>(
    config: Option<&ConstructionHeuristicConfig>,
    model: &ModelContext<S, V, DM, IDM>,
) -> bool
where
    S: PlanningSolution,
{
    let entity_class = config.and_then(|cfg| cfg.target.entity_class.as_deref());
    let variable_name = config.and_then(|cfg| cfg.target.variable_name.as_deref());
    let explicit_target = config.is_some_and(has_explicit_target);

    model
        .scalar_variables()
        .any(|ctx| !explicit_target || ctx.matches_target(entity_class, variable_name))
}

impl<S, V, DM, IDM, D, ProgressCb> Phase<S, D, ProgressCb> for ListRoundRobinPhase<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Send + Sync + Debug + 'static,
    DM: Clone + Send,
    IDM: Clone + Send,
    D: solverforge_scoring::Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);
        let solution = phase_scope.score_director().working_solution();
        let n_elements = (self.ctx.element_count)(solution);
        let n_entities = (self.ctx.entity_count)(solution);

        if n_entities == 0 || n_elements == 0 {
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned = (self.ctx.assigned_elements)(solution);
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

            let element = (self.ctx.index_to_element)(
                phase_scope.score_director().working_solution(),
                elem_idx,
            );
            if assigned_set.contains(&element) {
                continue;
            }

            let mut step_scope = StepScope::new(&mut phase_scope);
            step_scope.apply_committed_change(|score_director| {
                let insert_pos = (self.ctx.list_len)(score_director.working_solution(), entity_idx);
                score_director.before_variable_changed(self.ctx.descriptor_index, entity_idx);
                (self.ctx.list_insert)(
                    score_director.working_solution_mut(),
                    entity_idx,
                    insert_pos,
                    element,
                );
                score_director.after_variable_changed(self.ctx.descriptor_index, entity_idx);
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

pub struct Construction<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Debug + Send + 'static,
    IDM: Clone + Debug + Send + 'static,
{
    config: Option<ConstructionHeuristicConfig>,
    descriptor: SolutionDescriptor,
    model: ModelContext<S, V, DM, IDM>,
}

impl<S, V, DM, IDM> Construction<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Debug + Send + 'static,
    IDM: Clone + Debug + Send + 'static,
{
    fn new(
        config: Option<ConstructionHeuristicConfig>,
        descriptor: SolutionDescriptor,
        model: ModelContext<S, V, DM, IDM>,
    ) -> Self {
        Self {
            config,
            descriptor,
            model,
        }
    }

    fn solve_list<D, ProgressCb>(
        &self,
        solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
        list_variables: &[crate::builder::ListVariableContext<S, V, DM, IDM>],
    ) -> bool
    where
        D: solverforge_scoring::Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        let Some(config) = self.config.as_ref() else {
            panic!("specialized list construction requires explicit configuration");
        };
        if list_variables.is_empty() {
            panic!("list construction configured against a scalar-only context");
        }

        let mut ran_phase = false;
        for ctx in list_variables {
            if !list_work_remaining(ctx, solver_scope.working_solution()) {
                continue;
            }
            match config.construction_heuristic_type {
                ConstructionHeuristicType::ListRoundRobin => {
                    ListRoundRobinPhase { ctx: ctx.clone() }.solve(solver_scope);
                }
                ConstructionHeuristicType::ListCheapestInsertion => {
                    ListCheapestInsertionPhase::new(
                        ctx.element_count,
                        ctx.assigned_elements,
                        ctx.entity_count,
                        ctx.list_len,
                        ctx.list_insert,
                        ctx.construction_list_remove,
                        ctx.index_to_element,
                        ctx.descriptor_index,
                    )
                    .solve(solver_scope);
                }
                ConstructionHeuristicType::ListRegretInsertion => {
                    ListRegretInsertionPhase::new(
                        ctx.element_count,
                        ctx.assigned_elements,
                        ctx.entity_count,
                        ctx.list_len,
                        ctx.list_insert,
                        ctx.construction_list_remove,
                        ctx.index_to_element,
                        ctx.descriptor_index,
                    )
                    .solve(solver_scope);
                }
                ConstructionHeuristicType::ListClarkeWright => {
                    let (Some(depot), Some(dist), Some(load), Some(cap), Some(assign)) = (
                        ctx.cw_depot_fn,
                        ctx.cw_distance_fn,
                        ctx.cw_element_load_fn,
                        ctx.cw_capacity_fn,
                        ctx.cw_assign_route_fn,
                    ) else {
                        panic!(
                            "list_clarke_wright requires depot_fn, distance_fn, element_load_fn, capacity_fn, and assign_route_fn"
                        );
                    };
                    ListClarkeWrightPhase::new(
                        ctx.element_count,
                        ctx.assigned_elements,
                        ctx.entity_count,
                        ctx.list_len,
                        assign,
                        ctx.index_to_element,
                        depot,
                        dist,
                        load,
                        cap,
                        ctx.merge_feasible_fn,
                        ctx.descriptor_index,
                    )
                    .solve(solver_scope);
                }
                ConstructionHeuristicType::ListKOpt => {
                    let (Some(get_route), Some(set_route), Some(ko_depot), Some(ko_dist)) = (
                        ctx.k_opt_get_route,
                        ctx.k_opt_set_route,
                        ctx.k_opt_depot_fn,
                        ctx.k_opt_distance_fn,
                    ) else {
                        panic!(
                            "list_k_opt requires k_opt_get_route, k_opt_set_route, k_opt_depot_fn, and k_opt_distance_fn"
                        );
                    };
                    ListKOptPhase::<S, V>::new(
                        config.k,
                        ctx.entity_count,
                        get_route,
                        set_route,
                        ko_depot,
                        ko_dist,
                        ctx.k_opt_feasible_fn,
                        ctx.descriptor_index,
                    )
                    .solve(solver_scope);
                }
                other => panic!(
                    "list construction heuristic {:?} configured against a list variable",
                    other
                ),
            }
            ran_phase = true;
        }
        ran_phase
    }
}

impl<S, V, DM, IDM> Debug for Construction<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Debug + Send + 'static,
    IDM: Clone + Debug + Send + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Construction")
            .field("config", &self.config)
            .field("variable_count", &self.model.variables().len())
            .finish()
    }
}

impl<S, V, DM, IDM, D, ProgressCb> Phase<S, D, ProgressCb> for Construction<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    S::Score: Score + Copy,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    D: solverforge_scoring::Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        let config = self.config.as_ref();
        let heuristic = config
            .map(|cfg| cfg.construction_heuristic_type)
            .unwrap_or(ConstructionHeuristicType::FirstFit);
        let list_variables = matching_list_variables(config, &self.model);
        let has_matching_scalar = has_matching_scalar_target(config, &self.model);
        let explicit_target = config.is_some_and(has_explicit_target);

        if is_list_only_heuristic(heuristic) {
            assert!(
                self.model.has_list_variables(),
                "list construction heuristic {:?} configured against a solution with no planning list variable",
                heuristic
            );
            assert!(
                !explicit_target || !list_variables.is_empty(),
                "list construction heuristic {:?} does not match the targeted planning list variable for entity_class={:?} variable_name={:?}",
                heuristic,
                config.and_then(|cfg| cfg.target.entity_class.as_deref()),
                config.and_then(|cfg| cfg.target.variable_name.as_deref()),
            );

            let ran_child_phase = self.solve_list(solver_scope, &list_variables);
            if !ran_child_phase {
                finalize_noop_construction(solver_scope);
            }
            return;
        }

        if is_standard_only_heuristic(heuristic) {
            assert!(
                !explicit_target || has_matching_scalar,
                "standard construction heuristic {:?} does not match targeted standard planning variables for entity_class={:?} variable_name={:?}",
                heuristic,
                config.and_then(|cfg| cfg.target.entity_class.as_deref()),
                config.and_then(|cfg| cfg.target.variable_name.as_deref()),
            );
            let standard_remaining = standard_work_remaining_with_frontier(
                &self.descriptor,
                solver_scope.construction_frontier(),
                solver_scope.solution_revision(),
                if explicit_target {
                    config.and_then(|cfg| cfg.target.entity_class.as_deref())
                } else {
                    None
                },
                if explicit_target {
                    config.and_then(|cfg| cfg.target.variable_name.as_deref())
                } else {
                    None
                },
                solver_scope.working_solution(),
            );
            if standard_remaining {
                build_descriptor_construction(config, &self.descriptor).solve(solver_scope);
            } else {
                finalize_noop_construction(solver_scope);
            }
            return;
        }

        let ran_child_phase = crate::phase::construction::solve_unified_construction(
            config,
            &self.model,
            solver_scope,
        );
        if !ran_child_phase {
            finalize_noop_construction(solver_scope);
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "Construction"
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
) -> PhaseSequence<
    RuntimePhase<Construction<S, V, DM, IDM>, LocalSearch<S, V, DM, IDM>, Vnd<S, V, DM, IDM>>,
>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
{
    let mut phases = Vec::new();

    if config.phases.is_empty() {
        phases.push(RuntimePhase::Construction(Construction::new(
            None,
            descriptor.clone(),
            model.clone(),
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
                    model.clone(),
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
