use std::fmt::{self, Debug};
use std::hash::Hash;

use solverforge_config::{PhaseConfig, SolverConfig};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::MoveSelector;
use crate::phase::localsearch::{Acceptor, LocalSearchForager, LocalSearchPhase};
use crate::phase::{Phase, PhaseSequence};
use crate::runtime::{build_phases, Construction, RuntimePhase};
use crate::scope::{ProgressCallback, SolverScope};

use super::{LocalSearchStrategy, RuntimeModel};

mod custom;
pub(crate) mod defaults;

pub use custom::{CustomPhaseNode, CustomSearchPhase, NoCustomPhase, NoCustomPhases};

pub struct SearchContext<
    S,
    V = usize,
    DM = crate::heuristic::selector::DefaultCrossEntityDistanceMeter,
    IDM = crate::heuristic::selector::DefaultCrossEntityDistanceMeter,
> where
    S: PlanningSolution,
{
    descriptor: SolutionDescriptor,
    model: RuntimeModel<S, V, DM, IDM>,
    random_seed: Option<u64>,
}

impl<S, V, DM, IDM> SearchContext<S, V, DM, IDM>
where
    S: PlanningSolution,
{
    pub fn new(
        descriptor: SolutionDescriptor,
        model: RuntimeModel<S, V, DM, IDM>,
        random_seed: Option<u64>,
    ) -> Self {
        Self {
            descriptor,
            model,
            random_seed,
        }
    }

    pub fn descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    pub fn model(&self) -> &RuntimeModel<S, V, DM, IDM> {
        &self.model
    }

    pub fn seed(&self) -> Option<u64> {
        self.random_seed
    }

    pub fn defaults(self) -> SearchBuilder<S, V, DM, IDM, NoCustomPhases> {
        SearchBuilder {
            context: self,
            custom_phases: NoCustomPhases,
        }
    }
}

pub trait Search<
    S,
    V = usize,
    DM = crate::heuristic::selector::DefaultCrossEntityDistanceMeter,
    IDM = crate::heuristic::selector::DefaultCrossEntityDistanceMeter,
> where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
{
    type Phase<D, ProgressCb>: Phase<S, D, ProgressCb> + Debug + Send
    where
        D: solverforge_scoring::Director<S>,
        ProgressCb: ProgressCallback<S>;

    fn build<D, ProgressCb>(
        self,
        config: &SolverConfig,
    ) -> PhaseSequence<Self::Phase<D, ProgressCb>>
    where
        D: solverforge_scoring::Director<S>,
        ProgressCb: ProgressCallback<S>;
}

pub struct SearchBuilder<S, V, DM, IDM, CustomPhases>
where
    S: PlanningSolution,
{
    context: SearchContext<S, V, DM, IDM>,
    custom_phases: CustomPhases,
}

impl<S, V, DM, IDM, CustomPhases> SearchBuilder<S, V, DM, IDM, CustomPhases>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    CustomPhases: custom::CustomPhaseRegistry<S, V, DM, IDM>,
{
    pub fn phase<P, F>(
        self,
        name: &'static str,
        builder: F,
    ) -> SearchBuilder<S, V, DM, IDM, CustomPhaseNode<CustomPhases, F, P>>
    where
        F: Fn(&SearchContext<S, V, DM, IDM>) -> P + Send + Sync + 'static,
        P: CustomSearchPhase<S> + 'static,
    {
        assert!(
            !self.custom_phases.contains(name),
            "custom phase `{name}` was registered more than once",
        );
        SearchBuilder {
            context: self.context,
            custom_phases: CustomPhaseNode::new(self.custom_phases, name, builder),
        }
    }
}

impl<S, V, DM, IDM, CustomPhases> Search<S, V, DM, IDM>
    for SearchBuilder<S, V, DM, IDM, CustomPhases>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    CustomPhases: custom::CustomPhaseRegistry<S, V, DM, IDM>,
{
    type Phase<D, ProgressCb>
        = SearchRuntimePhase<
        RuntimePhase<Construction<S, V, DM, IDM>, LocalSearchStrategy<S, V, DM, IDM>>,
        CustomPhases::Phase,
    >
    where
        D: solverforge_scoring::Director<S>,
        ProgressCb: ProgressCallback<S>;

    fn build<D, ProgressCb>(
        self,
        config: &SolverConfig,
    ) -> PhaseSequence<Self::Phase<D, ProgressCb>>
    where
        D: solverforge_scoring::Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        let mut phases = Vec::new();
        if config.phases.is_empty() {
            for phase in
                build_phases(config, &self.context.descriptor, &self.context.model).into_phases()
            {
                phases.push(SearchRuntimePhase::Builtin(phase));
            }
            return PhaseSequence::new(phases);
        }

        for phase in &config.phases {
            match phase {
                PhaseConfig::Custom(custom) => {
                    let phase = self
                        .custom_phases
                        .build_named(&custom.name, &self.context)
                        .unwrap_or_else(|| {
                            panic!(
                                "custom phase `{}` was not registered by the solution search function",
                                custom.name
                            )
                        });
                    phases.push(SearchRuntimePhase::Custom(phase));
                }
                PhaseConfig::ConstructionHeuristic(_)
                | PhaseConfig::LocalSearch(_)
                | PhaseConfig::ExhaustiveSearch(_)
                | PhaseConfig::PartitionedSearch(_) => {
                    let mut single = config.clone();
                    single.phases = vec![phase.clone()];
                    let mut built =
                        build_phases(&single, &self.context.descriptor, &self.context.model)
                            .into_phases();
                    assert_eq!(
                        built.len(),
                        1,
                        "built-in phase expansion must produce one phase"
                    );
                    phases.push(SearchRuntimePhase::Builtin(built.remove(0)));
                }
            }
        }
        PhaseSequence::new(phases)
    }
}

pub enum SearchRuntimePhase<Builtin, Custom> {
    Builtin(Builtin),
    Custom(Custom),
}

impl<Builtin: Debug, Custom: Debug> Debug for SearchRuntimePhase<Builtin, Custom> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Builtin(phase) => f
                .debug_tuple("SearchRuntimePhase::Builtin")
                .field(phase)
                .finish(),
            Self::Custom(phase) => f
                .debug_tuple("SearchRuntimePhase::Custom")
                .field(phase)
                .finish(),
        }
    }
}

impl<S, D, ProgressCb, Builtin, Custom> Phase<S, D, ProgressCb>
    for SearchRuntimePhase<Builtin, Custom>
where
    S: PlanningSolution,
    D: solverforge_scoring::Director<S>,
    ProgressCb: ProgressCallback<S>,
    Builtin: Phase<S, D, ProgressCb>,
    Custom: CustomSearchPhase<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        match self {
            Self::Builtin(phase) => phase.solve(solver_scope),
            Self::Custom(phase) => CustomSearchPhase::solve(phase, solver_scope),
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "SearchRuntimePhase"
    }
}

pub fn build_search<S, V, DM, IDM, D, ProgressCb, T>(
    search: T,
    config: &SolverConfig,
) -> PhaseSequence<T::Phase<D, ProgressCb>>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    D: solverforge_scoring::Director<S>,
    ProgressCb: ProgressCallback<S>,
    T: Search<S, V, DM, IDM>,
    T::Phase<D, ProgressCb>: Phase<S, D, ProgressCb>,
{
    search.build::<D, ProgressCb>(config)
}

pub fn local_search<S, M, MS, A, Fo>(
    move_selector: MS,
    acceptor: A,
    forager: Fo,
) -> LocalSearchPhase<S, M, MS, A, Fo>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    MS: MoveSelector<S, M>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    LocalSearchPhase::new(move_selector, acceptor, forager, None)
}

#[cfg(test)]
mod tests;
