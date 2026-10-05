use super::Operator;
use crate::stream::relational::{DenseRowStore, HandleMap, RowHandle};

#[derive(Clone, Copy)]
struct BranchRow {
    right: bool,
    input: RowHandle,
}
/// Typed bag union: equal values in different branches keep distinct identities.
pub struct MergeNode<L, R> {
    left: L,
    right: R,
    rows: DenseRowStore<BranchRow>,
    left_rows: HandleMap<RowHandle>,
    right_rows: HandleMap<RowHandle>,
}
impl<L, R> MergeNode<L, R> {
    pub fn new(left: L, right: R) -> Self {
        Self {
            left,
            right,
            rows: DenseRowStore::new(),
            left_rows: HandleMap::new(),
            right_rows: HandleMap::new(),
        }
    }
    fn add(&mut self, input: RowHandle, right: bool) -> RowHandle {
        let h = self.rows.insert(BranchRow { right, input });
        if right {
            self.right_rows.insert(input, h);
        } else {
            self.left_rows.insert(input, h);
        }
        h
    }
}
impl<S: 'static, L, R> Operator<S> for MergeNode<L, R>
where
    L: Operator<S>,
    for<'a> R: Operator<S, View<'a> = L::View<'a>>,
{
    type View<'a> = L::View<'a>;
    type Evaluation = (L::Evaluation, R::Evaluation);
    #[inline]
    fn prepare_evaluation(&self, solution: &S) -> Self::Evaluation {
        (
            self.left.prepare_evaluation(solution),
            self.right.prepare_evaluation(solution),
        )
    }
    #[inline]
    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        self.left.visit_evaluation(solution, &evaluation.0, visitor);
        self.right
            .visit_evaluation(solution, &evaluation.1, visitor);
    }
    fn clear(&mut self) {
        self.left.clear();
        self.right.clear();
        self.rows.clear();
        self.left_rows.clear();
        self.right_rows.clear();
    }
    fn initialize(&mut self, solution: &S) {
        self.left.initialize(solution);
        self.right.initialize(solution);
        self.rows.clear();
        self.left_rows.clear();
        self.right_rows.clear();
        for h in self.left.handles() {
            self.add(h, false);
        }
        for h in self.right.handles() {
            self.add(h, true);
        }
    }
    fn handles(&self) -> Vec<RowHandle> {
        self.rows.iter().map(|(h, _)| h).collect()
    }
    fn resolve<'a>(&'a self, solution: &'a S, h: RowHandle) -> Option<Self::View<'a>> {
        let row = self.rows.get(h)?;
        if row.right {
            self.right.resolve(solution, row.input)
        } else {
            self.left.resolve(solution, row.input)
        }
    }
    fn visit_provenance(&self, h: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        if let Some(row) = self.rows.get(h) {
            if row.right {
                self.right.visit_provenance(row.input, visitor);
            } else {
                self.left.visit_provenance(row.input, visitor);
            }
        }
    }
    fn retract(&mut self, solution: &S, descriptor: usize, index: usize) -> Vec<RowHandle> {
        let left = self.left.retract(solution, descriptor, index);
        let right = self.right.retract(solution, descriptor, index);
        let mut removed = Vec::new();
        for (right, inputs) in [(false, left), (true, right)] {
            for input in inputs {
                let h = if right {
                    self.right_rows.remove(input)
                } else {
                    self.left_rows.remove(input)
                };
                if let Some(h) = h {
                    self.rows.retract(h);
                    removed.push(h);
                }
            }
        }
        removed
    }
    fn insert(&mut self, solution: &S, descriptor: usize, index: usize) -> Vec<RowHandle> {
        let left = self.left.insert(solution, descriptor, index);
        let right = self.right.insert(solution, descriptor, index);
        let mut inserted = Vec::new();
        for (right, inputs) in [(false, left), (true, right)] {
            for input in inputs {
                inserted.push(self.add(input, right));
            }
        }
        inserted
    }
}
