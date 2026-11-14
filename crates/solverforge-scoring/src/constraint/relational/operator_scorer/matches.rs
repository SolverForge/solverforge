//! Match listing for one shared-terminal scorer: the rows constraint
//! analysis reports and counts.

use solverforge_core::score::Score;

use super::{OperatorScorer, RowWeight};
use crate::api::analysis::{ConstraintJustification, DetailedConstraintMatch};
use crate::api::constraint_set::scale_contribution;
use crate::stream::relational::operator::{ExplainOperator, Operator, RowOrigin};

impl<W, Sc: Score> OperatorScorer<W, Sc> {
    /// Visits each row of `operator` with its origin and signed score: the
    /// retained rows and scores once the operator holds rows, a full
    /// evaluation of `solution` before that, like
    /// [`OperatorScorerSet::evaluate`].
    fn visit_scored<S, O>(
        &self,
        operator: &O,
        solution: &S,
        mut visit: impl FnMut(&O::View<'_>, RowOrigin<'_, O::Evaluation>, Sc),
    ) where
        S: 'static,
        O: Operator<S>,
        W: RowWeight<S, O, Sc>,
    {
        let handles = operator.handles();
        if handles.is_empty() {
            let evaluation = operator.prepare_evaluation(solution);
            operator.visit_evaluation(solution, &evaluation, &mut |row| {
                visit(
                    &row,
                    RowOrigin::Evaluation(&evaluation),
                    self.signed(self.weight.score(solution, &row)),
                );
            });
        } else {
            for handle in handles {
                let row = operator
                    .resolve(solution, handle)
                    .expect("live shared terminal row");
                let weight = match self.scores.get(handle) {
                    Some(weight) => *weight,
                    None => self.weight.score(solution, &row),
                };
                visit(&row, RowOrigin::Retained(handle), self.signed(weight));
            }
        }
    }

    /// One match per row of `operator` whose coded weight is not zero,
    /// scored at `multiplier` and justified by the entities the row was built
    /// from; the match scores sum to the terminal's total.
    pub(in crate::constraint::relational) fn matches<'a, S, O>(
        &'a self,
        operator: &O,
        solution: &S,
        multiplier: u32,
    ) -> Vec<DetailedConstraintMatch<'a, Sc>>
    where
        S: 'static,
        O: ExplainOperator<S>,
        W: RowWeight<S, O, Sc>,
    {
        let mut matches = Vec::new();
        self.visit_scored(operator, solution, |row, origin, score| {
            if score == Sc::zero() {
                return;
            }
            let score = scale_contribution(score, multiplier);
            let mut entities = Vec::new();
            operator.explain(solution, row, origin, &mut entities);
            matches.push(DetailedConstraintMatch::new(
                &self.constraint_ref,
                score,
                ConstraintJustification::new(entities),
            ));
        });
        matches
    }
}
