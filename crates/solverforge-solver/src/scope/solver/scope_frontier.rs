/* Construction-frontier and working-solution revision bookkeeping of the
solver scope. Included by `scope/solver.rs`. */

impl<'t, S: PlanningSolution, D: Director<S>, ProgressCb: ProgressCallback<S>>
    SolverScope<'t, S, D, ProgressCb>
{
    pub(crate) fn is_scalar_slot_completed(&self, slot_id: ConstructionSlotId) -> bool {
        self.construction_frontier
            .is_scalar_completed(slot_id, self.solution_revision)
    }

    pub(crate) fn mark_scalar_slot_completed(&mut self, slot_id: ConstructionSlotId) {
        self.construction_frontier
            .mark_scalar_completed(slot_id, self.solution_revision);
    }

    pub(crate) fn is_group_slot_completed(&self, slot_id: &ConstructionGroupSlotId) -> bool {
        self.construction_frontier
            .is_group_completed(slot_id, self.solution_revision)
    }

    pub(crate) fn mark_group_slot_completed(&mut self, slot_id: ConstructionGroupSlotId) {
        self.construction_frontier
            .mark_group_completed(slot_id, self.solution_revision);
    }

    pub(crate) fn is_list_element_completed(&self, element_id: ConstructionListElementId) -> bool {
        self.construction_frontier
            .is_list_completed(element_id, self.solution_revision)
    }

    pub(crate) fn mark_list_element_completed(&mut self, element_id: ConstructionListElementId) {
        self.construction_frontier
            .mark_list_completed(element_id, self.solution_revision);
    }

    #[cfg(test)]
    pub(crate) fn solution_revision(&self) -> u64 {
        self.solution_revision
    }

    pub(crate) fn working_solution_generation(&self) -> u64 {
        self.working_solution_generation
    }

    pub(crate) fn apply_committed_move<M>(&mut self, mov: &M)
    where
        M: Move<S>,
    {
        self.committed_mutation(|score_director| mov.do_move(score_director));
    }

    pub(crate) fn apply_committed_change<F>(&mut self, change: F)
    where
        F: FnOnce(&mut D),
    {
        self.committed_mutation(change);
    }
}
