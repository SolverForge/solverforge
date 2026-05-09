use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::MoveSelector;
use crate::phase::localsearch::{Acceptor, LocalSearchForager, LocalSearchPhase};
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

    fn phase_type_name(&self) -> &'static str {
        "CustomSearchPhase"
    }
}

impl<S, M, MS, A, Fo> CustomSearchPhase<S> for LocalSearchPhase<S, M, MS, A, Fo>
where
    S: PlanningSolution,
    M: Move<S>,
    MS: MoveSelector<S, M> + Debug,
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
}

pub trait CustomPhaseRegistry<S, V, DM, IDM>
where
    S: PlanningSolution,
{
    type Phase: CustomSearchPhase<S>;

    fn contains(&self, name: &str) -> bool;

    fn build_named(
        &self,
        name: &str,
        context: &SearchContext<S, V, DM, IDM>,
    ) -> Option<Self::Phase>;
}

pub struct NoCustomPhases;

impl<S, V, DM, IDM> CustomPhaseRegistry<S, V, DM, IDM> for NoCustomPhases
where
    S: PlanningSolution,
{
    type Phase = NoCustomPhase;

    fn contains(&self, _name: &str) -> bool {
        false
    }

    fn build_named(
        &self,
        _name: &str,
        _context: &SearchContext<S, V, DM, IDM>,
    ) -> Option<Self::Phase> {
        None
    }
}

#[derive(Clone, Copy)]
pub enum NoCustomPhase {}

impl Debug for NoCustomPhase {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl<S> CustomSearchPhase<S> for NoCustomPhase
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

impl<S, D, ProgressCb> Phase<S, D, ProgressCb> for NoCustomPhase
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
}

impl<S, V, DM, IDM, Previous, Builder, CurrentPhase> CustomPhaseRegistry<S, V, DM, IDM>
    for CustomPhaseNode<Previous, Builder, CurrentPhase>
where
    S: PlanningSolution,
    Previous: CustomPhaseRegistry<S, V, DM, IDM>,
    Builder: Fn(&SearchContext<S, V, DM, IDM>) -> CurrentPhase,
    CurrentPhase: CustomSearchPhase<S>,
{
    type Phase = CustomPhaseUnion<Previous::Phase, CurrentPhase>;

    fn contains(&self, name: &str) -> bool {
        self.name == name || self.previous.contains(name)
    }

    fn build_named(
        &self,
        name: &str,
        context: &SearchContext<S, V, DM, IDM>,
    ) -> Option<Self::Phase> {
        if self.name == name {
            return Some(CustomPhaseUnion::Current((self.builder)(context)));
        }
        self.previous
            .build_named(name, context)
            .map(CustomPhaseUnion::Previous)
    }
}
