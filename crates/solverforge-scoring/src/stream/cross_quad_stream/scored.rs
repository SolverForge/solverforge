use super::*;

/* Scored quad operator: the tree plus the authored filter as one row producer. */
pub struct QuadScored<
    S,
    A,
    B,
    C,
    D,
    EA,
    EB,
    EC,
    ED,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    P3: CompileCondition<Plan: IndexedPlan>,
    F,
> where
    BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>: Operator<S>,
    F: QuadFilter<S, A, B, C, D>,
{
    pub(super) inner: BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>,
    pub(super) filter: F,
    pub(super) accepted: HandleMap<()>,
    /// Child notification scratch, reused across events.
    pub(super) child_removed: Vec<RowHandle>,
    pub(super) child_inserted: Vec<RowHandle>,
    pub(super) marker: PhantomData<fn() -> (S, A, B, C, D)>,
}

impl<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F> Operator<S>
    for QuadScored<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    D: Clone + Send + Sync + 'static,
    P1: CompileCondition<Plan: IndexedPlan> + 'static,
    P2: CompileCondition<Plan: IndexedPlan> + 'static,
    P3: CompileCondition<Plan: IndexedPlan> + 'static,
    for<'a> BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>: Operator<
        S,
        View<'a> = Pair<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>, Leaf<'a, D>>,
    >,
    F: QuadFilter<S, A, B, C, D> + 'static,
{
    type View<'a> = QuadRow<'a, S, EA, EB, EC, ED, P1, P2, P3>;
    type Evaluation = <BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3> as Operator<S>>::Evaluation;

    #[inline]
    fn prepare_evaluation(&self, solution: &S) -> Self::Evaluation {
        self.inner.prepare_evaluation(solution)
    }

    #[inline]
    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        self.inner
            .visit_evaluation(solution, evaluation, &mut |row| {
                let (a, b, c, d) = row.entities();
                let (ai, bi, ci, di) = row.indexes();
                if self.filter.test(solution, a, b, c, d, ai, bi, ci, di) {
                    visitor(row);
                }
            });
    }

    fn clear(&mut self) {
        self.inner.clear();
        self.accepted.clear();
        self.child_removed.clear();
        self.child_inserted.clear();
    }

    fn initialize(&mut self, solution: &S) {
        self.inner.initialize(solution);
        self.accepted.clear();
        self.child_removed.clear();
        self.child_inserted.clear();
        for handle in self.inner.handles() {
            let row = self.inner.resolve(solution, handle).expect("live quad row");
            if self.test(solution, row) {
                self.accepted.insert(handle, ());
            }
        }
    }

    fn handles(&self) -> Vec<RowHandle> {
        self.inner
            .handles()
            .into_iter()
            .filter(|h| self.accepted.get(*h).is_some())
            .collect()
    }

    fn resolve<'a>(&'a self, solution: &'a S, handle: RowHandle) -> Option<Self::View<'a>> {
        self.accepted.get(handle)?;
        self.inner.resolve(solution, handle)
    }

    fn visit_provenance(&self, handle: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        if self.accepted.get(handle).is_some() {
            self.inner.visit_provenance(handle, visitor);
        }
    }

    fn retract(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        removed: &mut Vec<RowHandle>,
        _inserted: &mut Vec<RowHandle>,
    ) {
        removed.clear();
        let inner_removed = std::mem::take(&mut self.child_removed);
        let inner_inserted = std::mem::take(&mut self.child_inserted);
        let mut inner_removed = inner_removed;
        let mut inner_inserted = inner_inserted;
        inner_removed.clear();
        inner_inserted.clear();
        self.inner.retract(
            solution,
            descriptor,
            index,
            &mut inner_removed,
            &mut inner_inserted,
        );
        for h in inner_removed.drain(..) {
            if self.accepted.remove(h).is_some() {
                removed.push(h);
            }
        }
        self.child_removed = inner_removed;
        self.child_inserted = inner_inserted;
    }

    fn insert(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        _removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        let mut inner_removed = std::mem::take(&mut self.child_removed);
        let mut inner_inserted = std::mem::take(&mut self.child_inserted);
        inner_removed.clear();
        inner_inserted.clear();
        self.inner.insert(
            solution,
            descriptor,
            index,
            &mut inner_removed,
            &mut inner_inserted,
        );
        for handle in inner_inserted.drain(..) {
            let row = self
                .inner
                .resolve(solution, handle)
                .expect("live scored row");
            if self.test(solution, row) {
                self.accepted.insert(handle, ());
                inserted.push(handle);
            }
        }
        self.child_removed = inner_removed;
        self.child_inserted = inner_inserted;
    }
}

impl<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F>
    QuadScored<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F>
where
    S: Send + Sync + 'static,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    P3: CompileCondition<Plan: IndexedPlan>,
    BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>: Operator<S>,
    F: QuadFilter<S, A, B, C, D>,
{
    #[inline]
    fn test<'a>(
        &self,
        solution: &S,
        row: <BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3> as Operator<S>>::View<'a>,
    ) -> bool
    where
        A: 'a,
        B: 'a,
        C: 'a,
        D: 'a,
        <BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3> as Operator<S>>::View<'a>:
            QuadEntities<'a, A, B, C, D>,
    {
        let (a, b, c, d) = row.entities();
        let (ai, bi, ci, di) = row.indexes();
        self.filter.test(solution, a, b, c, d, ai, bi, ci, di)
    }
}
