use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_config::PartitionedSearchConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::phase::localsearch::MoveCursorSource;
use crate::phase::localsearch::{Acceptor, LocalSearchForager, LocalSearchPhase};
use crate::phase::partitioned::{ChildPhases, PartitionedSearchPhase, SolutionPartitioner};
use crate::phase::Phase;
use crate::scope::{ProgressCallback, SolverScope};

use super::SearchContext;

pub trait CustomSearchPhase<S>: Debug + Send
where
    S: PlanningSolution,
{
    fn solve<D, ProgressCb>(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>)
    where
        D: Director<S>,
        ProgressCb: ProgressCallback<S>;

    /// Runs once at the enclosing solver's terminal boundary.
    ///
    /// This mirrors [`Phase::on_solver_terminal`] for custom phases. The
    /// configured runtime dispatches it after instantiating the registered
    /// extension; implementations remain `CustomSearchPhase` rather than
    /// directly implementing `Phase`.
    fn on_solver_terminal<D, ProgressCb>(
        &mut self,
        _solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
    ) where
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
    }

    fn phase_type_name(&self) -> &'static str {
        "CustomSearchPhase"
    }
}

impl<S, M, Source, A, Fo> CustomSearchPhase<S> for LocalSearchPhase<S, M, Source, A, Fo>
where
    S: PlanningSolution,
    M: Move<S>,
    Source: MoveCursorSource<S, M> + Debug + Send,
    A: Acceptor<S> + Debug,
    Fo: LocalSearchForager<S, M> + Debug,
{
    fn solve<D, ProgressCb>(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>)
    where
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        Phase::solve(self, solver_scope);
    }

    fn phase_type_name(&self) -> &'static str {
        "LocalSearchPhase"
    }

    fn on_solver_terminal<D, ProgressCb>(
        &mut self,
        solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
    ) where
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        Phase::on_solver_terminal(self, solver_scope);
    }
}

impl<S, PD, Part, SDF, PF, CP> CustomSearchPhase<S>
    for PartitionedSearchPhase<S, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution + 'static,
    PD: Director<S> + 'static,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD> + Send,
{
    fn solve<D, ProgressCb>(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>)
    where
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        Phase::solve(self, solver_scope);
    }

    fn phase_type_name(&self) -> &'static str {
        "PartitionedSearch"
    }

    fn on_solver_terminal<D, ProgressCb>(
        &mut self,
        solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
    ) where
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        Phase::on_solver_terminal(self, solver_scope);
    }
}

/// Policy carried by one concrete extension registry.
///
/// Typed models may register monomorphized custom or partitioned extensions.
/// Dynamic models carry a distinct empty registry so compilation can reject
/// those declarations without asking a host binding to emulate them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeExtensionPolicy {
    Typed,
    Dynamic,
}

/// Recursive, concrete extension registry owned by a compiled runtime graph.
///
/// Compilation consults only the registration predicates. Instantiation is
/// deferred to the graph executor, which creates fresh concrete phases from
/// its frozen [`SearchContext`] once for each solve.
pub trait RuntimeExtensionRegistry<S, V, DM, IDM>
where
    S: PlanningSolution,
{
    type Phase: CustomSearchPhase<S>;

    fn policy(&self) -> RuntimeExtensionPolicy;

    fn contains_custom(&self, name: &str) -> bool;

    fn instantiate_custom(
        &self,
        name: &str,
        context: &SearchContext<S, V, DM, IDM>,
    ) -> Option<Self::Phase>;

    fn contains_partitioned(&self, name: &str) -> bool;

    fn instantiate_partitioned(
        &self,
        name: &str,
        config: &PartitionedSearchConfig,
        context: &SearchContext<S, V, DM, IDM>,
    ) -> Option<Self::Phase>;
}

pub struct NoTypedExtensions;

impl<S, V, DM, IDM> RuntimeExtensionRegistry<S, V, DM, IDM> for NoTypedExtensions
where
    S: PlanningSolution,
{
    type Phase = NoRuntimeExtensionPhase;

    fn policy(&self) -> RuntimeExtensionPolicy {
        RuntimeExtensionPolicy::Typed
    }

    fn contains_custom(&self, _name: &str) -> bool {
        false
    }

    fn instantiate_custom(
        &self,
        _name: &str,
        _context: &SearchContext<S, V, DM, IDM>,
    ) -> Option<Self::Phase> {
        None
    }

    fn contains_partitioned(&self, _name: &str) -> bool {
        false
    }

    fn instantiate_partitioned(
        &self,
        _name: &str,
        _config: &PartitionedSearchConfig,
        _context: &SearchContext<S, V, DM, IDM>,
    ) -> Option<Self::Phase> {
        None
    }
}

/// Empty registry used by host-language dynamic models.
///
/// Its distinct policy makes unsupported custom and partitioned declarations
/// a compile-time graph error; no extension builder exists or is invoked.
pub struct NoDynamicExtensions;

impl<S, V, DM, IDM> RuntimeExtensionRegistry<S, V, DM, IDM> for NoDynamicExtensions
where
    S: PlanningSolution,
{
    type Phase = NoRuntimeExtensionPhase;

    fn policy(&self) -> RuntimeExtensionPolicy {
        RuntimeExtensionPolicy::Dynamic
    }

    fn contains_custom(&self, _name: &str) -> bool {
        false
    }

    fn instantiate_custom(
        &self,
        _name: &str,
        _context: &SearchContext<S, V, DM, IDM>,
    ) -> Option<Self::Phase> {
        None
    }

    fn contains_partitioned(&self, _name: &str) -> bool {
        false
    }

    fn instantiate_partitioned(
        &self,
        _name: &str,
        _config: &PartitionedSearchConfig,
        _context: &SearchContext<S, V, DM, IDM>,
    ) -> Option<Self::Phase> {
        None
    }
}

#[derive(Clone, Copy)]
pub enum NoRuntimeExtensionPhase {}

impl Debug for NoRuntimeExtensionPhase {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl<S> CustomSearchPhase<S> for NoRuntimeExtensionPhase
where
    S: PlanningSolution,
{
    fn solve<D, ProgressCb>(&mut self, _solver_scope: &mut SolverScope<'_, S, D, ProgressCb>)
    where
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        match *self {}
    }

    fn phase_type_name(&self) -> &'static str {
        match *self {}
    }
}

impl<S, D, ProgressCb> Phase<S, D, ProgressCb> for NoRuntimeExtensionPhase
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    fn solve(&mut self, _solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        match *self {}
    }

    fn phase_type_name(&self) -> &'static str {
        match *self {}
    }
}

pub struct CustomPhaseNode<Previous, Builder, Phase> {
    previous: Previous,
    name: &'static str,
    builder: Builder,
    _marker: PhantomData<fn() -> Phase>,
}

pub struct PartitionedPhaseNode<Previous, Builder, Phase> {
    previous: Previous,
    name: &'static str,
    builder: Builder,
    _marker: PhantomData<fn() -> Phase>,
}

impl<Previous, Builder, Phase> PartitionedPhaseNode<Previous, Builder, Phase> {
    pub fn new(previous: Previous, name: &'static str, builder: Builder) -> Self {
        Self {
            previous,
            name,
            builder,
            _marker: PhantomData,
        }
    }
}

impl<Previous, Builder, Phase> CustomPhaseNode<Previous, Builder, Phase> {
    pub fn new(previous: Previous, name: &'static str, builder: Builder) -> Self {
        Self {
            previous,
            name,
            builder,
            _marker: PhantomData,
        }
    }
}

pub enum CustomPhaseUnion<PreviousPhase, CurrentPhase> {
    Previous(PreviousPhase),
    Current(CurrentPhase),
}

impl<PreviousPhase: Debug, CurrentPhase: Debug> Debug
    for CustomPhaseUnion<PreviousPhase, CurrentPhase>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Previous(phase) => f.debug_tuple("CustomPhase::Previous").field(phase).finish(),
            Self::Current(phase) => f.debug_tuple("CustomPhase::Current").field(phase).finish(),
        }
    }
}

impl<S, PreviousPhase, CurrentPhase> CustomSearchPhase<S>
    for CustomPhaseUnion<PreviousPhase, CurrentPhase>
where
    S: PlanningSolution,
    PreviousPhase: CustomSearchPhase<S>,
    CurrentPhase: CustomSearchPhase<S>,
{
    fn solve<D, ProgressCb>(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>)
    where
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        match self {
            Self::Previous(phase) => phase.solve(solver_scope),
            Self::Current(phase) => phase.solve(solver_scope),
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "CustomPhase"
    }

    fn on_solver_terminal<D, ProgressCb>(
        &mut self,
        solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
    ) where
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        match self {
            Self::Previous(phase) => phase.on_solver_terminal(solver_scope),
            Self::Current(phase) => phase.on_solver_terminal(solver_scope),
        }
    }
}

impl<S, D, ProgressCb, PreviousPhase, CurrentPhase> Phase<S, D, ProgressCb>
    for CustomPhaseUnion<PreviousPhase, CurrentPhase>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    PreviousPhase: CustomSearchPhase<S>,
    CurrentPhase: CustomSearchPhase<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        CustomSearchPhase::solve(self, solver_scope);
    }

    fn phase_type_name(&self) -> &'static str {
        CustomSearchPhase::<S>::phase_type_name(self)
    }

    fn on_solver_terminal(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        CustomSearchPhase::on_solver_terminal(self, solver_scope);
    }
}

impl<S, V, DM, IDM, Previous, Builder, CurrentPhase> RuntimeExtensionRegistry<S, V, DM, IDM>
    for CustomPhaseNode<Previous, Builder, CurrentPhase>
where
    S: PlanningSolution,
    Previous: RuntimeExtensionRegistry<S, V, DM, IDM>,
    Builder: Fn(&SearchContext<S, V, DM, IDM>) -> CurrentPhase,
    CurrentPhase: CustomSearchPhase<S>,
{
    type Phase = CustomPhaseUnion<Previous::Phase, CurrentPhase>;

    fn policy(&self) -> RuntimeExtensionPolicy {
        self.previous.policy()
    }

    fn contains_custom(&self, name: &str) -> bool {
        self.name == name || self.previous.contains_custom(name)
    }

    fn instantiate_custom(
        &self,
        name: &str,
        context: &SearchContext<S, V, DM, IDM>,
    ) -> Option<Self::Phase> {
        if self.name == name {
            return Some(CustomPhaseUnion::Current((self.builder)(context)));
        }
        self.previous
            .instantiate_custom(name, context)
            .map(CustomPhaseUnion::Previous)
    }

    fn contains_partitioned(&self, name: &str) -> bool {
        self.previous.contains_partitioned(name)
    }

    fn instantiate_partitioned(
        &self,
        name: &str,
        config: &PartitionedSearchConfig,
        context: &SearchContext<S, V, DM, IDM>,
    ) -> Option<Self::Phase> {
        self.previous
            .instantiate_partitioned(name, config, context)
            .map(CustomPhaseUnion::Previous)
    }
}

impl<S, V, DM, IDM, Previous, Builder, CurrentPhase> RuntimeExtensionRegistry<S, V, DM, IDM>
    for PartitionedPhaseNode<Previous, Builder, CurrentPhase>
where
    S: PlanningSolution,
    Previous: RuntimeExtensionRegistry<S, V, DM, IDM>,
    Builder: Fn(&SearchContext<S, V, DM, IDM>, &PartitionedSearchConfig) -> CurrentPhase,
    CurrentPhase: CustomSearchPhase<S>,
{
    type Phase = CustomPhaseUnion<Previous::Phase, CurrentPhase>;

    fn policy(&self) -> RuntimeExtensionPolicy {
        self.previous.policy()
    }

    fn contains_custom(&self, name: &str) -> bool {
        self.previous.contains_custom(name)
    }

    fn instantiate_custom(
        &self,
        name: &str,
        context: &SearchContext<S, V, DM, IDM>,
    ) -> Option<Self::Phase> {
        self.previous
            .instantiate_custom(name, context)
            .map(CustomPhaseUnion::Previous)
    }

    fn contains_partitioned(&self, name: &str) -> bool {
        self.name == name || self.previous.contains_partitioned(name)
    }

    fn instantiate_partitioned(
        &self,
        name: &str,
        config: &PartitionedSearchConfig,
        context: &SearchContext<S, V, DM, IDM>,
    ) -> Option<Self::Phase> {
        if self.name == name {
            return Some(CustomPhaseUnion::Current((self.builder)(context, config)));
        }
        self.previous
            .instantiate_partitioned(name, config, context)
            .map(CustomPhaseUnion::Previous)
    }
}
