use std::fmt::{self, Debug};
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_config::{ConstructionHeuristicConfig, PhaseConfig, SolverConfig};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};

use crate::builder::{build_local_search, LocalSearchStrategy, RuntimeModel};
use crate::descriptor::{
    build_descriptor_construction_from_bindings, scalar_work_remaining_with_frontier,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::manager::solve_specialized_list_construction;
use crate::phase::construction::{select_construction_capabilities, ConstructionRoute};
use crate::phase::{sequence::PhaseSequence, Phase};
use crate::scope::{ProgressCallback, SolverScope};

#[path = "runtime/defaults.rs"]
mod defaults;

#[cfg(test)]
mod tests;

#[cfg(test)]
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

pub struct Construction<S, V, DM, IDM>
where
    S: PlanningSolution,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Debug + Send + 'static,
    IDM: Clone + Debug + Send + 'static,
{
    config: Option<ConstructionHeuristicConfig>,
    descriptor: SolutionDescriptor,
    model: RuntimeModel<S, V, DM, IDM>,
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
        model: RuntimeModel<S, V, DM, IDM>,
    ) -> Self {
        Self {
            config,
            descriptor,
            model,
        }
    }

    fn solve_list<D, ProgressCb>(
        &self,
        config: &ConstructionHeuristicConfig,
        solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
        list_variables: &[crate::builder::ListVariableSlot<S, V, DM, IDM>],
    ) -> bool
    where
        D: solverforge_scoring::Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        if list_variables.is_empty() {
            panic!("list construction configured against a scalar-only model");
        }

        solve_specialized_list_construction(
            config.construction_heuristic_type,
            config.k,
            solver_scope,
            list_variables,
        )
    }

    pub(super) fn solve_configured<D, ProgressCb>(
        &self,
        config: Option<&ConstructionHeuristicConfig>,
        solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
        required_only: bool,
    ) -> bool
    where
        S: PlanningSolution + 'static,
        S::Score: Score + Copy,
        DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
        IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
        D: solverforge_scoring::Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        let capabilities = select_construction_capabilities(config, &self.descriptor, &self.model);

        match capabilities.route {
            ConstructionRoute::SpecializedList => {
                let Some(config) = config else {
                    panic!("specialized list construction requires explicit configuration");
                };
                self.solve_list(config, solver_scope, &capabilities.list_variables)
            }
            ConstructionRoute::Descriptor => {
                let scalar_remaining = scalar_work_remaining_with_frontier(
                    &self.descriptor,
                    solver_scope.construction_frontier(),
                    solver_scope.solution_revision(),
                    capabilities.entity_class.as_deref(),
                    capabilities.variable_name.as_deref(),
                    solver_scope.working_solution(),
                );
                if scalar_remaining {
                    build_descriptor_construction_from_bindings(
                        config,
                        &self.descriptor,
                        capabilities.scalar_bindings.clone(),
                    )
                    .solve(solver_scope);
                    true
                } else {
                    false
                }
            }
            ConstructionRoute::GroupedScalar => {
                let Some((group_index, group)) = capabilities.scalar_group.as_ref() else {
                    unreachable!("grouped scalar route requires a selected scalar group");
                };
                if !scalar_group_work_remaining(group, solver_scope.working_solution()) {
                    return false;
                }
                record_scalar_assignment_remaining(group, solver_scope);
                let mut phase = crate::phase::construction::build_scalar_group_construction(
                    config,
                    *group_index,
                    group.clone(),
                    capabilities.scalar_bindings.clone(),
                    required_only,
                );
                phase.solve(solver_scope);
                record_scalar_assignment_remaining(group, solver_scope);
                true
            }
            ConstructionRoute::GenericMixed => {
                crate::phase::construction::solve_construction(config, &self.model, solver_scope)
            }
        }
    }
}

fn scalar_group_work_remaining<S>(
    group: &crate::builder::ScalarGroupBinding<S>,
    solution: &S,
) -> bool {
    if let Some(assignment) = group.assignment() {
        return assignment.unassigned_count(solution) > 0;
    }
    group.members.iter().any(|member| {
        (0..member.entity_count(solution))
            .any(|entity_index| member.current_value(solution, entity_index).is_none())
    })
}

fn record_scalar_assignment_remaining<S, D, ProgressCb>(
    group: &crate::builder::ScalarGroupBinding<S>,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) where
    S: PlanningSolution,
    D: solverforge_scoring::Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    if let Some(assignment) = group.assignment() {
        let remaining = assignment.remaining_required_count(solver_scope.working_solution());
        solver_scope
            .stats_mut()
            .record_scalar_assignment_required_remaining(group.group_name, remaining);
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
        let ran_child_phase = match self.config.as_ref() {
            None => defaults::solve_default_construction(self, solver_scope),
            Some(config) => self.solve_configured(Some(config), solver_scope, false),
        };
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

pub enum RuntimePhase<C, LS> {
    Construction(C),
    LocalSearch(LS),
}

impl<C, LS> Debug for RuntimePhase<C, LS>
where
    C: Debug,
    LS: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Construction(phase) => write!(f, "RuntimePhase::Construction({phase:?})"),
            Self::LocalSearch(phase) => write!(f, "RuntimePhase::LocalSearch({phase:?})"),
        }
    }
}

impl<S, D, ProgressCb, C, LS> Phase<S, D, ProgressCb> for RuntimePhase<C, LS>
where
    S: PlanningSolution,
    D: solverforge_scoring::Director<S>,
    ProgressCb: ProgressCallback<S>,
    C: Phase<S, D, ProgressCb> + Debug,
    LS: Phase<S, D, ProgressCb> + Debug,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        match self {
            Self::Construction(phase) => phase.solve(solver_scope),
            Self::LocalSearch(phase) => phase.solve(solver_scope),
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "RuntimePhase"
    }
}

pub fn build_phases<S, V, DM, IDM>(
    config: &SolverConfig,
    descriptor: &SolutionDescriptor,
    model: &RuntimeModel<S, V, DM, IDM>,
) -> PhaseSequence<RuntimePhase<Construction<S, V, DM, IDM>, LocalSearchStrategy<S, V, DM, IDM>>>
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
        for phase in
            crate::builder::search::defaults::default_local_search_phases(model, config.random_seed)
        {
            phases.push(RuntimePhase::LocalSearch(phase));
        }
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
            PhaseConfig::Custom(custom) => {
                panic!(
                    "custom phase `{}` requires a typed solution search function",
                    custom.name
                );
            }
            PhaseConfig::PartitionedSearch(partitioned) => {
                let name = partitioned
                    .partitioner
                    .as_deref()
                    .unwrap_or("<missing partitioner>");
                panic!(
                    "partitioned_search partitioner `{name}` requires typed partitioner registration"
                );
            }
        }
    }

    PhaseSequence::new(phases)
}

#[cfg(test)]
mod construction_routing_tests {
    use solverforge_config::ConstructionHeuristicType;

    fn should_use_descriptor_path(
        heuristic: ConstructionHeuristicType,
        has_scalar_variables: bool,
        has_list_variables: bool,
    ) -> bool {
        if !has_scalar_variables {
            return false;
        }

        match heuristic {
            ConstructionHeuristicType::ListRoundRobin
            | ConstructionHeuristicType::ListCheapestInsertion
            | ConstructionHeuristicType::ListRegretInsertion
            | ConstructionHeuristicType::ListClarkeWright
            | ConstructionHeuristicType::ListKOpt => false,
            ConstructionHeuristicType::FirstFit | ConstructionHeuristicType::CheapestInsertion => {
                !has_list_variables
            }
            ConstructionHeuristicType::FirstFitDecreasing
            | ConstructionHeuristicType::WeakestFit
            | ConstructionHeuristicType::WeakestFitDecreasing
            | ConstructionHeuristicType::StrongestFit
            | ConstructionHeuristicType::StrongestFitDecreasing
            | ConstructionHeuristicType::AllocateEntityFromQueue
            | ConstructionHeuristicType::AllocateToValueFromQueue => true,
        }
    }

    #[test]
    fn pure_scalar_first_fit_uses_descriptor_path() {
        assert!(should_use_descriptor_path(
            ConstructionHeuristicType::FirstFit,
            true,
            false,
        ));
    }

    #[test]
    fn pure_scalar_cheapest_insertion_uses_descriptor_path() {
        assert!(should_use_descriptor_path(
            ConstructionHeuristicType::CheapestInsertion,
            true,
            false,
        ));
    }

    #[test]
    fn mixed_first_fit_keeps_generic_construction_path() {
        assert!(!should_use_descriptor_path(
            ConstructionHeuristicType::FirstFit,
            true,
            true,
        ));
    }

    #[test]
    fn mixed_cheapest_insertion_keeps_generic_construction_path() {
        assert!(!should_use_descriptor_path(
            ConstructionHeuristicType::CheapestInsertion,
            true,
            true,
        ));
    }

    #[test]
    fn scalar_only_heuristics_still_route_to_descriptor_path() {
        assert!(should_use_descriptor_path(
            ConstructionHeuristicType::StrongestFit,
            true,
            true,
        ));
    }
}
