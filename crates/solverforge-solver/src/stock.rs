use std::fmt::Debug;
use std::hash::Hash;

use solverforge_config::{ConstructionHeuristicConfig, PhaseConfig, SolverConfig};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};

use crate::builder::ListContext;
use crate::descriptor_standard::{
    build_descriptor_construction, build_descriptor_local_search, build_descriptor_vnd,
    DescriptorConstruction, DescriptorLocalSearch, DescriptorVnd, SeedBestSolutionPhase,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::list_solver::{build_list_construction, ListConstruction};
use crate::mixed_stock::{
    build_mixed_local_search, build_mixed_vnd, MixedStockLocalSearch, MixedStockVnd,
};
use crate::phase::{sequence::PhaseSequence, Phase};
use crate::scope::{ProgressCallback, SolverScope};

pub enum StockPhase<C, LS, VND> {
    Seed(SeedBestSolutionPhase),
    Construction(C),
    LocalSearch(LS),
    Vnd(VND),
}

impl<C, LS, VND> Debug for StockPhase<C, LS, VND>
where
    C: Debug,
    LS: Debug,
    VND: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Seed(phase) => write!(f, "StockPhase::Seed({phase:?})"),
            Self::Construction(phase) => write!(f, "StockPhase::Construction({phase:?})"),
            Self::LocalSearch(phase) => write!(f, "StockPhase::LocalSearch({phase:?})"),
            Self::Vnd(phase) => write!(f, "StockPhase::Vnd({phase:?})"),
        }
    }
}

impl<S, D, ProgressCb, C, LS, VND> Phase<S, D, ProgressCb> for StockPhase<C, LS, VND>
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
            Self::Seed(phase) => phase.solve(solver_scope),
            Self::Construction(phase) => phase.solve(solver_scope),
            Self::LocalSearch(phase) => phase.solve(solver_scope),
            Self::Vnd(phase) => phase.solve(solver_scope),
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "StockPhase"
    }
}

pub type StandardStockPhase<S> =
    StockPhase<DescriptorConstruction<S>, DescriptorLocalSearch<S>, DescriptorVnd<S>>;

pub type UnifiedMixedStockPhase<S, V, DM, IDM> = StockPhase<
    ListConstruction<S, V>,
    MixedStockLocalSearch<S, V, DM, IDM>,
    MixedStockVnd<S, V, DM, IDM>,
>;

#[derive(Clone, Copy)]
pub struct MixedStockConstructionArgs<S, V> {
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

pub fn build_standard_stock_phases<S>(
    config: &SolverConfig,
    descriptor: &SolutionDescriptor,
) -> PhaseSequence<StandardStockPhase<S>>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
{
    let mut phases = Vec::new();

    if config.phases.is_empty() {
        phases.push(StockPhase::Construction(build_descriptor_construction(
            None, descriptor,
        )));
        phases.push(StockPhase::LocalSearch(build_descriptor_local_search(
            None, descriptor,
        )));
        return PhaseSequence::new(phases);
    }

    let mut saw_seed = false;
    for phase in &config.phases {
        match phase {
            PhaseConfig::ConstructionHeuristic(ch) => {
                phases.push(StockPhase::Construction(build_descriptor_construction(
                    Some(ch),
                    descriptor,
                )));
            }
            PhaseConfig::LocalSearch(ls) => {
                if !saw_seed {
                    phases.push(StockPhase::Seed(SeedBestSolutionPhase));
                    saw_seed = true;
                }
                phases.push(StockPhase::LocalSearch(build_descriptor_local_search(
                    Some(ls),
                    descriptor,
                )));
            }
            PhaseConfig::Vnd(vnd) => {
                if !saw_seed {
                    phases.push(StockPhase::Seed(SeedBestSolutionPhase));
                    saw_seed = true;
                }
                phases.push(StockPhase::Vnd(build_descriptor_vnd(vnd, descriptor)));
            }
            _ => {
                panic!("unsupported stock phase in unified stock runtime");
            }
        }
    }

    PhaseSequence::new(phases)
}

pub fn build_mixed_stock_phases<S, V, DM, IDM>(
    config: &SolverConfig,
    descriptor: &SolutionDescriptor,
    list_ctx: &ListContext<S, V, DM, IDM>,
    construction: MixedStockConstructionArgs<S, V>,
    list_variable_name: &'static str,
) -> PhaseSequence<UnifiedMixedStockPhase<S, V, DM, IDM>>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + Hash + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    let mut phases = Vec::new();

    if config.phases.is_empty() {
        phases.push(StockPhase::Seed(SeedBestSolutionPhase));
        phases.push(StockPhase::Construction(build_list_construction(
            None,
            construction.element_count,
            construction.assigned_elements,
            construction.entity_count,
            construction.list_len,
            construction.list_insert,
            construction.list_remove,
            construction.index_to_element,
            construction.descriptor_index,
            construction.depot_fn,
            construction.distance_fn,
            construction.element_load_fn,
            construction.capacity_fn,
            construction.assign_route_fn,
            construction.merge_feasible_fn,
            construction.k_opt_get_route,
            construction.k_opt_set_route,
            construction.k_opt_depot_fn,
            construction.k_opt_distance_fn,
            construction.k_opt_feasible_fn,
        )));
        phases.push(StockPhase::LocalSearch(build_mixed_local_search(
            None, descriptor, list_ctx,
        )));
        return PhaseSequence::new(phases);
    }

    let mut saw_seed = false;
    for phase in &config.phases {
        match phase {
            PhaseConfig::ConstructionHeuristic(ch) => {
                validate_mixed_construction_target(ch, list_variable_name);
                if !saw_seed {
                    phases.push(StockPhase::Seed(SeedBestSolutionPhase));
                    saw_seed = true;
                }
                phases.push(StockPhase::Construction(build_list_construction(
                    Some(ch),
                    construction.element_count,
                    construction.assigned_elements,
                    construction.entity_count,
                    construction.list_len,
                    construction.list_insert,
                    construction.list_remove,
                    construction.index_to_element,
                    construction.descriptor_index,
                    construction.depot_fn,
                    construction.distance_fn,
                    construction.element_load_fn,
                    construction.capacity_fn,
                    construction.assign_route_fn,
                    construction.merge_feasible_fn,
                    construction.k_opt_get_route,
                    construction.k_opt_set_route,
                    construction.k_opt_depot_fn,
                    construction.k_opt_distance_fn,
                    construction.k_opt_feasible_fn,
                )));
            }
            PhaseConfig::LocalSearch(ls) => {
                if !saw_seed {
                    phases.push(StockPhase::Seed(SeedBestSolutionPhase));
                    saw_seed = true;
                }
                phases.push(StockPhase::LocalSearch(build_mixed_local_search(
                    Some(ls),
                    descriptor,
                    list_ctx,
                )));
            }
            PhaseConfig::Vnd(vnd) => {
                if !saw_seed {
                    phases.push(StockPhase::Seed(SeedBestSolutionPhase));
                    saw_seed = true;
                }
                phases.push(StockPhase::Vnd(build_mixed_vnd(vnd, descriptor, list_ctx)));
            }
            _ => {
                panic!("unsupported stock phase in unified stock runtime");
            }
        }
    }

    PhaseSequence::new(phases)
}

fn validate_mixed_construction_target(
    config: &ConstructionHeuristicConfig,
    list_variable_name: &'static str,
) {
    if let Some(variable_name) = config.target.variable_name.as_deref() {
        if variable_name != list_variable_name {
            panic!(
                "construction heuristic targeting standard variables is not implemented in the unified stock path yet"
            );
        }
    } else if config.target.entity_class.is_some() {
        panic!(
            "construction heuristic entity_class targeting is not implemented in the unified stock path yet"
        );
    }
}
