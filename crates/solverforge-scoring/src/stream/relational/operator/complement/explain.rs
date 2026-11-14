// Cold explanation of complement rows: a real row by its target and its
// result row, a defaulted key by its target alone.
use super::super::{ComplementView, ExplainOperator, RowOrigin};
use super::{ComplementNode, OutputValue};
use crate::api::analysis::EntityRef;
use crate::stream::joiner::plan::{ExecutablePlan, IndexedPlan};
use crate::stream::relational::keyed_plan::{KeyOf, RightKeyOf};

impl<
        S: 'static,
        L: ExplainOperator<S>,
        R: ExplainOperator<S>,
        P: IndexedPlan + KeyOf + RightKeyOf<Key = <P as KeyOf>::Key> + 'static,
        F: 'static,
        T: 'static,
    > ExplainOperator<S> for ComplementNode<S, L, R, P, F, T>
where
    for<'a> P: ExecutablePlan<L::View<'a>, R::View<'a>>,
    F: for<'a> Fn(&S, &L::View<'a>) -> T,
{
    fn explain(
        &self,
        solution: &S,
        row: &Self::View<'_>,
        origin: RowOrigin<'_, Self::Evaluation>,
        entities: &mut Vec<EntityRef>,
    ) {
        let (target, result) = match origin {
            RowOrigin::Retained(handle) => match self.rows.get(handle).map(|row| &row.value) {
                Some(OutputValue::Real(id)) => (
                    RowOrigin::Retained(id.left()),
                    Some(RowOrigin::Retained(id.right())),
                ),
                Some(OutputValue::Default(input, _)) => (RowOrigin::Retained(*input), None),
                None => return,
            },
            RowOrigin::Evaluation(evaluation) => (
                RowOrigin::Evaluation(&evaluation.left.0),
                Some(RowOrigin::Evaluation(&evaluation.left.1)),
            ),
        };
        match row {
            ComplementView::Real(pair) => {
                self.targets.explain(solution, &pair.left, target, entities);
                if let Some(result) = result {
                    self.results
                        .explain(solution, &pair.right, result, entities);
                }
            }
            ComplementView::Default(default) => {
                self.targets
                    .explain(solution, &default.input, target, entities);
            }
        }
    }
}
