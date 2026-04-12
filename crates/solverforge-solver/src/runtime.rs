use std::fmt::{self, Debug};
use std::hash::Hash;

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, PhaseConfig, SolverConfig,
};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};

use crate::builder::ListContext;
use crate::descriptor_standard::{
    build_descriptor_construction, standard_target_matches, standard_work_remaining,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::list_solver::build_list_construction;
use crate::phase::{sequence::PhaseSequence, Phase};
use crate::scope::{ProgressCallback, SolverScope};
use crate::unified_search::{
    build_unified_local_search, build_unified_vnd, UnifiedLocalSearch, UnifiedVnd,
};

pub struct UnifiedConstruction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + 'static,
{
    config: Option<ConstructionHeuristicConfig>,
    descriptor: SolutionDescriptor,
    list_construction: Option<ListConstructionArgs<S, V>>,
    list_variable_name: Option<&'static str>,
}

impl<S, V> UnifiedConstruction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + 'static,
{
    fn new(
        config: Option<ConstructionHeuristicConfig>,
        descriptor: SolutionDescriptor,
        list_construction: Option<ListConstructionArgs<S, V>>,
        list_variable_name: Option<&'static str>,
    ) -> Self {
        Self {
            config,
            descriptor,
            list_construction,
            list_variable_name,
        }
    }
}

impl<S, V> Debug for UnifiedConstruction<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnifiedConstruction")
            .field("config", &self.config)
            .field("has_list_construction", &self.list_construction.is_some())
            .field("list_variable_name", &self.list_variable_name)
            .finish()
    }
}

impl<S, V, D, ProgressCb> Phase<S, D, ProgressCb> for UnifiedConstruction<S, V>
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
        let list_matches = config.is_some_and(|cfg| {
            list_target_matches(
                cfg,
                &self.descriptor,
                self.list_construction.as_ref(),
                self.list_variable_name,
            )
        });

        if let Some(cfg) = config {
            if explicit_target && !standard_matches && !list_matches {
                panic!(
                    "construction heuristic matched no planning variables for entity_class={:?} variable_name={:?}",
                    cfg.target.entity_class,
                    cfg.target.variable_name
                );
            }

            let heuristic = cfg.construction_heuristic_type;
            if is_list_only_heuristic(heuristic) {
                assert!(
                    self.list_construction.is_some(),
                    "list construction heuristic {:?} configured against a solution with no planning list variable",
                    heuristic
                );
                assert!(
                    !explicit_target || list_matches,
                    "list construction heuristic {:?} does not match the targeted planning list variable for entity_class={:?} variable_name={:?}",
                    heuristic,
                    cfg.target.entity_class,
                    cfg.target.variable_name
                );
                self.solve_list(solver_scope);
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
                build_descriptor_construction(Some(cfg), &self.descriptor).solve(solver_scope);
                return;
            }
        }

        if self.list_construction.is_none() {
            build_descriptor_construction(config, &self.descriptor).solve(solver_scope);
            return;
        }

        let standard_remaining = standard_work_remaining(
            &self.descriptor,
            if explicit_target { entity_class } else { None },
            if explicit_target { variable_name } else { None },
            solver_scope.working_solution(),
        );
        let list_remaining = self
            .list_construction
            .as_ref()
            .map(|args| {
                (!explicit_target || list_matches)
                    && list_work_remaining(args, solver_scope.working_solution())
            })
            .unwrap_or(false);

        if standard_remaining {
            build_descriptor_construction(config, &self.descriptor).solve(solver_scope);
        }
        if list_remaining {
            self.solve_list(solver_scope);
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "UnifiedConstruction"
    }
}

impl<S, V> UnifiedConstruction<S, V>
where
    S: PlanningSolution + 'static,
    V: Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
{
    fn solve_list<D, ProgressCb>(&self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>)
    where
        D: solverforge_scoring::Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        let Some(args) = self.list_construction.as_ref() else {
            panic!("list construction configured against a standard-variable context");
        };
        let normalized = normalize_list_construction_config(self.config.as_ref());
        build_list_construction(
            normalized.as_ref(),
            args.element_count,
            args.assigned_elements,
            args.entity_count,
            args.list_len,
            args.list_insert,
            args.list_remove,
            args.index_to_element,
            args.descriptor_index,
            args.depot_fn,
            args.distance_fn,
            args.element_load_fn,
            args.capacity_fn,
            args.assign_route_fn,
            args.merge_feasible_fn,
            args.k_opt_get_route,
            args.k_opt_set_route,
            args.k_opt_depot_fn,
            args.k_opt_distance_fn,
            args.k_opt_feasible_fn,
        )
        .solve(solver_scope);
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

pub type UnifiedRuntimePhase<S, V, DM, IDM> = RuntimePhase<
    UnifiedConstruction<S, V>,
    UnifiedLocalSearch<S, V, DM, IDM>,
    UnifiedVnd<S, V, DM, IDM>,
>;

pub struct ListConstructionArgs<S, V> {
    pub element_count: fn(&S) -> usize,
    pub assigned_elements: fn(&S) -> Vec<V>,
    pub entity_count: fn(&S) -> usize,
    pub list_len: fn(&S, usize) -> usize,
    pub list_insert: fn(&mut S, usize, usize, V),
    pub list_remove: fn(&mut S, usize, usize) -> V,
    pub index_to_element: fn(&S, usize) -> V,
    pub descriptor_index: usize,
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

impl<S, V> Clone for ListConstructionArgs<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> Copy for ListConstructionArgs<S, V> {}

pub fn build_phases<S, V, DM, IDM>(
    config: &SolverConfig,
    descriptor: &SolutionDescriptor,
    list_ctx: Option<&ListContext<S, V, DM, IDM>>,
    list_construction: Option<ListConstructionArgs<S, V>>,
    list_variable_name: Option<&'static str>,
) -> PhaseSequence<UnifiedRuntimePhase<S, V, DM, IDM>>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    let mut phases = Vec::new();

    if config.phases.is_empty() {
        phases.push(default_construction_phase(
            descriptor,
            list_construction.as_ref(),
            list_variable_name,
        ));
        phases.push(RuntimePhase::LocalSearch(build_unified_local_search(
            None,
            descriptor,
            list_ctx,
            config.random_seed,
        )));
        return PhaseSequence::new(phases);
    }

    for phase in &config.phases {
        match phase {
            PhaseConfig::ConstructionHeuristic(ch) => {
                phases.push(build_construction_phase(
                    ch,
                    descriptor,
                    list_construction.as_ref(),
                    list_variable_name,
                ));
            }
            PhaseConfig::LocalSearch(ls) => {
                phases.push(RuntimePhase::LocalSearch(build_unified_local_search(
                    Some(ls),
                    descriptor,
                    list_ctx,
                    config.random_seed,
                )));
            }
            PhaseConfig::Vnd(vnd) => {
                phases.push(RuntimePhase::Vnd(build_unified_vnd(
                    vnd,
                    descriptor,
                    list_ctx,
                    config.random_seed,
                )));
            }
            _ => {
                panic!("unsupported phase in the unified runtime");
            }
        }
    }

    PhaseSequence::new(phases)
}

fn default_construction_phase<S, V, DM, IDM>(
    descriptor: &SolutionDescriptor,
    list_construction: Option<&ListConstructionArgs<S, V>>,
    list_variable_name: Option<&'static str>,
) -> UnifiedRuntimePhase<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    RuntimePhase::Construction(UnifiedConstruction::new(
        None,
        descriptor.clone(),
        list_construction.copied(),
        list_variable_name,
    ))
}

fn build_construction_phase<S, V, DM, IDM>(
    config: &ConstructionHeuristicConfig,
    descriptor: &SolutionDescriptor,
    list_construction: Option<&ListConstructionArgs<S, V>>,
    list_variable_name: Option<&'static str>,
) -> UnifiedRuntimePhase<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    RuntimePhase::Construction(UnifiedConstruction::new(
        Some(config.clone()),
        descriptor.clone(),
        list_construction.copied(),
        list_variable_name,
    ))
}

fn list_work_remaining<S, V>(args: &ListConstructionArgs<S, V>, solution: &S) -> bool
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
    descriptor: &SolutionDescriptor,
    list_construction: Option<&ListConstructionArgs<S, V>>,
    list_variable_name: Option<&'static str>,
) -> bool
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Send + Sync + 'static,
{
    if !has_explicit_target(config) {
        return false;
    }

    let Some(list_variable_name) = list_variable_name else {
        return false;
    };
    let Some(list_construction) = list_construction else {
        return false;
    };
    let Some(list_entity_name) = descriptor
        .entity_descriptors
        .get(list_construction.descriptor_index)
        .map(|entity| entity.type_name)
    else {
        return false;
    };

    config
        .target
        .variable_name
        .as_deref()
        .is_none_or(|name| name == list_variable_name)
        && config
            .target
            .entity_class
            .as_deref()
            .is_none_or(|name| name == list_entity_name)
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

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_config::VariableTargetConfig;
    use solverforge_core::domain::{EntityDescriptor, VariableDescriptor, VariableType};
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct TestSolution {
        score: Option<solverforge_core::score::SoftScore>,
    }

    impl PlanningSolution for TestSolution {
        type Score = solverforge_core::score::SoftScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn standard_variable(name: &'static str) -> VariableDescriptor {
        VariableDescriptor {
            name,
            variable_type: VariableType::Genuine,
            allows_unassigned: true,
            value_range_provider: Some("values"),
            value_range_type: solverforge_core::domain::ValueRangeType::Collection,
            source_variable: None,
            source_entity: None,
            usize_getter: Some(|_| None),
            usize_setter: Some(|_, _| {}),
            entity_value_provider: Some(|_| vec![1]),
        }
    }

    fn descriptor() -> SolutionDescriptor {
        SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
            .with_entity(
                EntityDescriptor::new("Route", TypeId::of::<()>(), "routes")
                    .with_variable(standard_variable("vehicle_id"))
                    .with_variable(VariableDescriptor::list("visits")),
            )
            .with_entity(
                EntityDescriptor::new("Shift", TypeId::of::<u8>(), "shifts")
                    .with_variable(standard_variable("employee_id")),
            )
    }

    fn list_args() -> ListConstructionArgs<TestSolution, usize> {
        ListConstructionArgs {
            element_count: |_| 0,
            assigned_elements: |_| Vec::new(),
            entity_count: |_| 0,
            list_len: |_, _| 0,
            list_insert: |_, _, _, _| {},
            list_remove: |_, _, _| 0,
            index_to_element: |_, _| 0,
            descriptor_index: 0,
            depot_fn: None,
            distance_fn: None,
            element_load_fn: None,
            capacity_fn: None,
            assign_route_fn: None,
            merge_feasible_fn: None,
            k_opt_get_route: None,
            k_opt_set_route: None,
            k_opt_depot_fn: None,
            k_opt_distance_fn: None,
            k_opt_feasible_fn: None,
        }
    }

    fn config(
        construction_heuristic_type: ConstructionHeuristicType,
        entity_class: Option<&str>,
        variable_name: Option<&str>,
    ) -> ConstructionHeuristicConfig {
        ConstructionHeuristicConfig {
            construction_heuristic_type,
            target: VariableTargetConfig {
                entity_class: entity_class.map(str::to_owned),
                variable_name: variable_name.map(str::to_owned),
            },
            k: 2,
            termination: None,
        }
    }

    #[test]
    fn list_target_requires_matching_variable_name() {
        let descriptor = descriptor();
        let cfg = config(
            ConstructionHeuristicType::ListCheapestInsertion,
            Some("Shift"),
            Some("employee_id"),
        );
        assert!(!list_target_matches(
            &cfg,
            &descriptor,
            Some(&list_args()),
            Some("visits")
        ));
    }

    #[test]
    fn list_target_matches_entity_class_only_for_owner() {
        let descriptor = descriptor();
        let cfg = config(
            ConstructionHeuristicType::ListCheapestInsertion,
            Some("Route"),
            None,
        );
        assert!(list_target_matches(
            &cfg,
            &descriptor,
            Some(&list_args()),
            Some("visits")
        ));
    }

    #[test]
    fn generic_list_dispatch_normalizes_to_list_cheapest_insertion() {
        let cfg = config(
            ConstructionHeuristicType::FirstFit,
            Some("Route"),
            Some("visits"),
        );
        let normalized = normalize_list_construction_config(Some(&cfg))
            .expect("generic list config should normalize");
        assert_eq!(
            normalized.construction_heuristic_type,
            ConstructionHeuristicType::ListCheapestInsertion
        );
    }

    #[test]
    fn standard_target_matches_entity_class_only_target() {
        let descriptor = descriptor();
        assert!(standard_target_matches(&descriptor, Some("Route"), None,));
    }
}
