use super::{ExplainRow, Operator};
use crate::api::analysis::EntityRef;
use crate::stream::collector::Accumulator;
use crate::stream::relational::RowHandle;
use std::marker::PhantomData;

pub(super) enum Contributors<'a, S: 'static, O: Operator<S>> {
    Retained(&'a [RowHandle]),
    Evaluation(&'a O::Evaluation, &'a [usize]),
}
impl<S: 'static, O: Operator<S>> Copy for Contributors<'_, S, O> {}
impl<S: 'static, O: Operator<S>> Clone for Contributors<'_, S, O> {
    fn clone(&self) -> Self {
        *self
    }
}
/// A borrowed group key and accumulator-owned result, with complete input lineage.
/// Use `with_result` to borrow collector results without cloning them.
pub struct GroupView<'a, S: 'static, O: Operator<S>, K, A, V, R> {
    pub key: &'a K,
    pub(super) accumulator: &'a A,
    pub(super) input: &'a O,
    pub(super) solution: &'a S,
    pub(super) contributors: Contributors<'a, S, O>,
    pub(super) marker: PhantomData<fn() -> (V, R)>,
}
impl<S: 'static, O: Operator<S>, K, A, V, R> Copy for GroupView<'_, S, O, K, A, V, R> {}
impl<S: 'static, O: Operator<S>, K, A, V, R> Clone for GroupView<'_, S, O, K, A, V, R> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<S: 'static, O: Operator<S>, K, A: Accumulator<V, R>, V, R> GroupView<'_, S, O, K, A, V, R> {
    pub fn with_result<T>(&self, f: impl FnOnce(&R) -> T) -> T {
        self.accumulator.with_result(f)
    }
}
impl<S: 'static, O: Operator<S>, K, A, V, R> ExplainRow for GroupView<'_, S, O, K, A, V, R>
where
    for<'a> O::View<'a>: ExplainRow,
{
    fn explain(&self, entities: &mut Vec<EntityRef>) {
        match self.contributors {
            Contributors::Retained(handles) => {
                for &h in handles {
                    self.input
                        .resolve(self.solution, h)
                        .expect("live group contributor")
                        .explain(entities);
                }
            }
            Contributors::Evaluation(evaluation, ordinals) => {
                let mut ordinal = 0;
                self.input
                    .visit_evaluation(self.solution, evaluation, &mut |row| {
                        if ordinals.contains(&ordinal) {
                            row.explain(entities);
                        }
                        ordinal += 1;
                    });
            }
        }
    }
}
