use solverforge_config::ConstructionObligation;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use super::forager::{
    BestFitForager, ConstructionChoice, ConstructionForager, FirstFeasibleForager, FirstFitForager,
    StrongestFitForager, WeakestFitForager,
};
use super::forager_step::{
    select_best_fit_index, select_first_feasible_index, select_first_fit_index,
    select_strongest_fit_index, select_weakest_fit_index,
};
use super::Placement;
use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::MoveCursor;
use crate::scope::{ProgressCallback, StepScope};

impl<S, M> ConstructionForager<S, M> for FirstFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn select_move_index<D, BestCb, C>(
        &self,
        placement: &mut Placement<S, M, C>,
        construction_obligation: ConstructionObligation,
        step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    ) -> Option<ConstructionChoice>
    where
        D: Director<S>,
        BestCb: ProgressCallback<S>,
        C: MoveCursor<S, M>,
    {
        select_first_fit_index(placement, construction_obligation, step_scope)
    }
}

impl<S, M> ConstructionForager<S, M> for BestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn select_move_index<D, BestCb, C>(
        &self,
        placement: &mut Placement<S, M, C>,
        construction_obligation: ConstructionObligation,
        step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    ) -> Option<ConstructionChoice>
    where
        D: Director<S>,
        BestCb: ProgressCallback<S>,
        C: MoveCursor<S, M>,
    {
        select_best_fit_index(placement, construction_obligation, step_scope)
    }
}

impl<S, M> ConstructionForager<S, M> for FirstFeasibleForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn select_move_index<D, BestCb, C>(
        &self,
        placement: &mut Placement<S, M, C>,
        construction_obligation: ConstructionObligation,
        step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    ) -> Option<ConstructionChoice>
    where
        D: Director<S>,
        BestCb: ProgressCallback<S>,
        C: MoveCursor<S, M>,
    {
        select_first_feasible_index(placement, construction_obligation, step_scope)
    }
}

impl<S, M> ConstructionForager<S, M> for WeakestFitForager<S, M>
where
    S: PlanningSolution,
    S::Score: Score,
    M: Move<S>,
{
    fn select_move_index<D, BestCb, C>(
        &self,
        placement: &mut Placement<S, M, C>,
        construction_obligation: ConstructionObligation,
        step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    ) -> Option<ConstructionChoice>
    where
        D: Director<S>,
        BestCb: ProgressCallback<S>,
        C: MoveCursor<S, M>,
    {
        select_weakest_fit_index(self, placement, construction_obligation, step_scope)
    }
}

impl<S, M> ConstructionForager<S, M> for StrongestFitForager<S, M>
where
    S: PlanningSolution,
    S::Score: Score,
    M: Move<S>,
{
    fn select_move_index<D, BestCb, C>(
        &self,
        placement: &mut Placement<S, M, C>,
        construction_obligation: ConstructionObligation,
        step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    ) -> Option<ConstructionChoice>
    where
        D: Director<S>,
        BestCb: ProgressCallback<S>,
        C: MoveCursor<S, M>,
    {
        select_strongest_fit_index(self, placement, construction_obligation, step_scope)
    }
}
