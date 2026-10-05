use super::{ExplainRow, Operator, RowChanges};
use crate::api::analysis::EntityRef;
use crate::stream::relational::{DenseRowStore, HandleMap, RowHandle};
use std::marker::PhantomData;

/// A borrowed child with its complete owning row and child position.
pub struct FlattenView<'a, V, T> {
    pub input: V,
    pub value: &'a T,
    pub child: usize,
}
impl<V: Copy, T> Copy for FlattenView<'_, V, T> {}
impl<V: Copy, T> Clone for FlattenView<'_, V, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<V: ExplainRow, T> ExplainRow for FlattenView<'_, V, T> {
    fn explain(&self, entities: &mut Vec<EntityRef>) {
        self.input.explain(entities);
    }
}
struct Child {
    input: RowHandle,
    child: usize,
}
/// Flatten borrowed child slices from arbitrary input rows; retention owns only identities.
pub struct FlattenNode<O, F, T> {
    input: O,
    extractor: F,
    rows: DenseRowStore<Child>,
    outputs: HandleMap<Vec<RowHandle>>,
    marker: PhantomData<fn() -> T>,
}
impl<O, F, T> FlattenNode<O, F, T> {
    pub fn new(input: O, extractor: F) -> Self {
        Self {
            input,
            extractor,
            rows: DenseRowStore::new(),
            outputs: HandleMap::new(),
            marker: PhantomData,
        }
    }
    fn apply_changes<S: 'static>(&mut self, solution: &S, changes: RowChanges) -> RowChanges
    where
        O: Operator<S>,
        F: for<'a> Fn(&'a S, O::View<'a>) -> &'a [T],
    {
        let mut removed = Vec::new();
        for input in changes.removed {
            for h in self.outputs.remove(input).unwrap_or_default() {
                self.rows.retract(h);
                removed.push(h);
            }
        }
        let mut inserted = Vec::new();
        for input in changes.inserted {
            let row = self
                .input
                .resolve(solution, input)
                .expect("inserted flattened input");
            let count = (self.extractor)(solution, row).len();
            let outputs = self.outputs.get_or_insert_with(input, Vec::new);
            for child in 0..count {
                let h = self.rows.insert(Child { input, child });
                outputs.push(h);
                inserted.push(h);
            }
        }
        RowChanges { removed, inserted }
    }
}
impl<S: 'static, O: Operator<S>, F, T: 'static> Operator<S> for FlattenNode<O, F, T>
where
    F: for<'a> Fn(&'a S, O::View<'a>) -> &'a [T] + 'static,
{
    type View<'a> = FlattenView<'a, O::View<'a>, T>;
    type Evaluation = O::Evaluation;
    fn prepare_evaluation(&self, solution: &S) -> Self::Evaluation {
        self.input.prepare_evaluation(solution)
    }
    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        self.input
            .visit_evaluation(solution, evaluation, &mut |input| {
                for (child, value) in (self.extractor)(solution, input).iter().enumerate() {
                    visitor(FlattenView {
                        input,
                        value,
                        child,
                    });
                }
            });
    }
    fn clear(&mut self) {
        self.input.clear();
        self.rows.clear();
        self.outputs.clear();
    }
    fn initialize(&mut self, solution: &S) {
        self.clear();
        self.input.initialize(solution);
        self.apply_changes(
            solution,
            RowChanges {
                removed: Vec::new(),
                inserted: self.input.handles(),
            },
        );
    }
    fn handles(&self) -> Vec<RowHandle> {
        self.rows.iter().map(|(h, _)| h).collect()
    }
    fn resolve<'a>(&'a self, solution: &'a S, h: RowHandle) -> Option<Self::View<'a>> {
        let child = self.rows.get(h)?;
        let input = self.input.resolve(solution, child.input)?;
        let value = (self.extractor)(solution, input).get(child.child)?;
        Some(FlattenView {
            input,
            value,
            child: child.child,
        })
    }
    fn visit_provenance(&self, h: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        if let Some(child) = self.rows.get(h) {
            self.input.visit_provenance(child.input, visitor);
        }
    }
    fn retract(&mut self, solution: &S, descriptor: usize, index: usize) -> RowChanges {
        let changes = self.input.retract(solution, descriptor, index);
        self.apply_changes(solution, changes)
    }
    fn insert(&mut self, solution: &S, descriptor: usize, index: usize) -> RowChanges {
        let changes = self.input.insert(solution, descriptor, index);
        self.apply_changes(solution, changes)
    }
}
