/* Runtime constraint weights on the score director.

Weightedness is a compile-time property of the solution type
(`PlanningSolution::HAS_CONSTRAINT_WEIGHTS`). For a type without weights every
check below is the constant `false`, so the director compiles to the
unweighted scoring calls with no binding work and no per-call branch. A
weighted type binds one multiplier per constraint weight slot on construction
and on every (re)initialization, and takes the weighted calls only while some
multiplier differs from `1`: one predictable branch per director call.
*/

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use super::descriptor::DirectorDescriptor;
use super::incremental::ScoreDirector;
use crate::api::constraint_set::ConstraintSet;

// Multipliers resolved by name for each weight slot of a constraint set.
#[derive(Default)]
pub(super) struct ConstraintWeightBinding {
    multipliers: Vec<u32>,
    nontrivial: bool,
}

impl ConstraintWeightBinding {
    pub(super) fn multipliers(&self) -> &[u32] {
        &self.multipliers
    }

    fn bind<S, C>(&mut self, solution: &S, constraints: &C)
    where
        S: PlanningSolution,
        C: ConstraintSet<S, S::Score>,
    {
        let mut refs = Vec::with_capacity(constraints.constraint_count());
        constraints.constraint_weight_refs(&mut refs);
        debug_assert_eq!(
            refs.len(),
            constraints.constraint_count(),
            "constraint set must expose one weight slot per constraint"
        );
        self.multipliers.clear();
        match solution.constraint_weights() {
            Some(weights) => self.multipliers.extend(
                refs.iter()
                    .map(|constraint| weights.multiplier_for(constraint)),
            ),
            None => self.multipliers.resize(refs.len(), 1),
        }
        self.nontrivial = self.multipliers.iter().any(|&multiplier| multiplier != 1);
    }
}

/* Entity-row deltas as the list-shadow bookkeeping applies them: the
unweighted constraint-set calls, or the weighted calls with bound multipliers. */
pub(super) trait RowDeltas<S, Sc> {
    fn insert_row(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;
    fn retract_row(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc;
}

// The unweighted calls, exactly as the constraint set makes them.
pub(super) struct UnweightedRows<'a, C>(pub(super) &'a mut C);

impl<S, Sc, C> RowDeltas<S, Sc> for UnweightedRows<'_, C>
where
    Sc: Score,
    C: ConstraintSet<S, Sc>,
{
    #[inline(always)]
    fn insert_row(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.0
            .on_insert_all(solution, entity_index, descriptor_index)
    }

    #[inline(always)]
    fn retract_row(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.0
            .on_retract_all(solution, entity_index, descriptor_index)
    }
}

struct WeightedRows<'a, C> {
    constraints: &'a mut C,
    multipliers: &'a [u32],
}

impl<S, Sc, C> RowDeltas<S, Sc> for WeightedRows<'_, C>
where
    Sc: Score,
    C: ConstraintSet<S, Sc>,
{
    fn insert_row(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.constraints.on_insert_all_weighted(
            solution,
            entity_index,
            descriptor_index,
            self.multipliers,
        )
    }

    fn retract_row(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.constraints.on_retract_all_weighted(
            solution,
            entity_index,
            descriptor_index,
            self.multipliers,
        )
    }
}

impl<S, C, Desc> ScoreDirector<S, C, Desc>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    Desc: DirectorDescriptor<S, C>,
{
    // Whether scoring must take the weighted calls; constant `false` without weights.
    #[inline(always)]
    pub(super) fn weighted(&self) -> bool {
        S::HAS_CONSTRAINT_WEIGHTS && self.weights.nontrivial
    }

    /* Resolves the working solution's multipliers for every weight slot.

    Runs on construction and before every (re)initialization, so a weight
    change made through `working_solution_mut()` takes effect after `reset()`.
    */
    pub(super) fn bind_constraint_weights(&mut self) {
        self.weights.bind(&self.working_solution, &self.constraints);
    }

    pub(super) fn before_variable_changed_weighted(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
    ) {
        let multipliers = self.weights.multipliers();
        let delta = self.constraints.on_retract_all_weighted(
            &self.working_solution,
            entity_index,
            descriptor_index,
            multipliers,
        );
        let element_delta = self.list_shadow_rows.retract_held(
            &self.working_solution,
            &mut WeightedRows {
                constraints: &mut self.constraints,
                multipliers,
            },
            descriptor_index,
            entity_index,
        );
        self.cached_score = self.cached_score + delta + element_delta;
    }

    pub(super) fn after_variable_changed_weighted(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
    ) {
        let delta = self.list_shadow_rows.refresh_owner(
            &mut self.working_solution,
            &mut WeightedRows {
                constraints: &mut self.constraints,
                multipliers: self.weights.multipliers(),
            },
            descriptor_index,
            entity_index,
        );
        self.cached_score = self.cached_score + delta;
    }
}
