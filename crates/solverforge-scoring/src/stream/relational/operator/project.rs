use super::{ExplainRow, Operator};
use crate::api::analysis::EntityRef;
use crate::stream::relational::{DenseRowStore, HandleMap, RowHandle};

/// Borrowed owned output with its entire upstream row and emission position.
pub struct ProjectView<'a, V, T> {
    pub input: V,
    pub value: &'a T,
    pub emission: usize,
}
impl<V: Copy, T> Copy for ProjectView<'_, V, T> {}
impl<V: Copy, T> Clone for ProjectView<'_, V, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<V: ExplainRow, T> ExplainRow for ProjectView<'_, V, T> {
    fn explain(&self, entities: &mut Vec<EntityRef>) {
        self.input.explain(entities);
    }
}

struct Output<T> {
    input: RowHandle,
    value: T,
    emission: usize,
}
/// Projection owns emitted values; no trait bounds are imposed on payloads.
pub struct ProjectNode<O, F, T> {
    input: O,
    mapper: F,
    rows: DenseRowStore<Output<T>>,
    outputs: HandleMap<Vec<RowHandle>>,
}
/// Fresh projection payloads with an independent owner for upstream derived values.
#[doc(hidden)]
pub struct ProjectEvaluation<E, T> {
    input: E,
    values: Vec<Vec<T>>,
}
impl<O, F, T> ProjectNode<O, F, T> {
    pub fn new(input: O, mapper: F) -> Self {
        Self {
            input,
            mapper,
            rows: DenseRowStore::new(),
            outputs: HandleMap::new(),
        }
    }
    fn add(&mut self, input: RowHandle, values: Vec<T>, inserted: &mut Vec<RowHandle>) {
        let outputs = self.outputs.get_or_insert_with(input, Vec::new);
        for (emission, value) in values.into_iter().enumerate() {
            let h = self.rows.insert(Output {
                input,
                value,
                emission,
            });
            outputs.push(h);
            inserted.push(h);
        }
    }
}
impl<S: 'static, O, F, T: 'static> Operator<S> for ProjectNode<O, F, T>
where
    O: Operator<S>,
    F: for<'a> Fn(&S, &O::View<'a>) -> Vec<T> + 'static,
{
    type View<'a> = ProjectView<'a, O::View<'a>, T>;
    type Evaluation = ProjectEvaluation<O::Evaluation, T>;
    fn prepare_evaluation(&self, solution: &S) -> Self::Evaluation {
        let input = self.input.prepare_evaluation(solution);
        let mut values = Vec::new();
        self.input.visit_evaluation(solution, &input, &mut |row| {
            values.push((self.mapper)(solution, &row));
        });
        ProjectEvaluation { input, values }
    }
    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        let mut ordinal = 0;
        self.input
            .visit_evaluation(solution, &evaluation.input, &mut |input| {
                for (emission, value) in evaluation.values[ordinal].iter().enumerate() {
                    visitor(ProjectView {
                        input,
                        value,
                        emission,
                    });
                }
                ordinal += 1;
            });
    }
    fn clear(&mut self) {
        self.input.clear();
        self.rows.clear();
        self.outputs.clear();
    }
    fn initialize(&mut self, solution: &S) {
        self.input.initialize(solution);
        self.rows.clear();
        self.outputs.clear();
        for h in self.input.handles() {
            let row = self
                .input
                .resolve(solution, h)
                .expect("live projected input");
            let values = (self.mapper)(solution, &row);
            self.add(h, values, &mut Vec::new());
        }
    }
    fn handles(&self) -> Vec<RowHandle> {
        self.rows.iter().map(|(h, _)| h).collect()
    }
    fn resolve<'a>(&'a self, solution: &'a S, h: RowHandle) -> Option<Self::View<'a>> {
        let row = self.rows.get(h)?;
        Some(ProjectView {
            input: self.input.resolve(solution, row.input)?,
            value: &row.value,
            emission: row.emission,
        })
    }
    fn visit_provenance(&self, h: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        if let Some(row) = self.rows.get(h) {
            self.input.visit_provenance(row.input, visitor);
        }
    }
    fn retract(&mut self, solution: &S, descriptor: usize, index: usize) -> Vec<RowHandle> {
        let inputs = self.input.retract(solution, descriptor, index);
        let mut removed = Vec::new();
        for input in inputs {
            for h in self.outputs.remove(input).unwrap_or_default() {
                self.rows.retract(h);
                removed.push(h);
            }
        }
        removed
    }
    fn insert(&mut self, solution: &S, descriptor: usize, index: usize) -> Vec<RowHandle> {
        let inputs = self.input.insert(solution, descriptor, index);
        let mut inserted = Vec::new();
        for input in inputs {
            let row = self
                .input
                .resolve(solution, input)
                .expect("inserted projected input");
            let values = (self.mapper)(solution, &row);
            self.add(input, values, &mut inserted);
        }
        inserted
    }
}
