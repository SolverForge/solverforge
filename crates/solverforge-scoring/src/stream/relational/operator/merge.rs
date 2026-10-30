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
    /// Child notification scratch, reused across events (L/R, removed/inserted).
    child_l_removed: Vec<RowHandle>,
    child_l_inserted: Vec<RowHandle>,
    child_r_removed: Vec<RowHandle>,
    child_r_inserted: Vec<RowHandle>,
}
impl<L, R> MergeNode<L, R> {
    /// Emits every row of `left` and every row of `right`, each as its own row.
    pub fn new(left: L, right: R) -> Self {
        Self {
            left,
            right,
            rows: DenseRowStore::new(),
            left_rows: HandleMap::new(),
            right_rows: HandleMap::new(),
            child_l_removed: Vec::new(),
            child_l_inserted: Vec::new(),
            child_r_removed: Vec::new(),
            child_r_inserted: Vec::new(),
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
impl<L, R> MergeNode<L, R> {
    fn apply_changes(
        &mut self,
        left_removed: &[RowHandle],
        left_inserted: &[RowHandle],
        right_removed: &[RowHandle],
        right_inserted: &[RowHandle],
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        for (right, removed_inputs, inserted_inputs) in [
            (false, left_removed, left_inserted),
            (true, right_removed, right_inserted),
        ] {
            for &input in removed_inputs {
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
            for &input in inserted_inputs {
                inserted.push(self.add(input, right));
            }
        }
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
        self.child_l_removed.clear();
        self.child_l_inserted.clear();
        self.child_r_removed.clear();
        self.child_r_inserted.clear();
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
    fn retract(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        removed.clear();
        inserted.clear();
        let mut lm = std::mem::take(&mut self.child_l_removed);
        let mut li = std::mem::take(&mut self.child_l_inserted);
        lm.clear();
        li.clear();
        self.left
            .retract(solution, descriptor, index, &mut lm, &mut li);
        let mut rm = std::mem::take(&mut self.child_r_removed);
        let mut ri = std::mem::take(&mut self.child_r_inserted);
        rm.clear();
        ri.clear();
        self.right
            .retract(solution, descriptor, index, &mut rm, &mut ri);
        self.apply_changes(&lm, &li, &rm, &ri, removed, inserted);
        self.child_l_removed = lm;
        self.child_l_inserted = li;
        self.child_r_removed = rm;
        self.child_r_inserted = ri;
    }
    fn insert(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        removed.clear();
        inserted.clear();
        let mut lm = std::mem::take(&mut self.child_l_removed);
        let mut li = std::mem::take(&mut self.child_l_inserted);
        lm.clear();
        li.clear();
        self.left
            .insert(solution, descriptor, index, &mut lm, &mut li);
        let mut rm = std::mem::take(&mut self.child_r_removed);
        let mut ri = std::mem::take(&mut self.child_r_inserted);
        rm.clear();
        ri.clear();
        self.right
            .insert(solution, descriptor, index, &mut rm, &mut ri);
        self.apply_changes(&lm, &li, &rm, &ri, removed, inserted);
        self.child_l_removed = lm;
        self.child_l_inserted = li;
        self.child_r_removed = rm;
        self.child_r_inserted = ri;
    }
}
