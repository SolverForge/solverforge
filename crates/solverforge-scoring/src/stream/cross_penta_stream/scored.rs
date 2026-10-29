use super::*;

/* Scored penta operator: the tree plus the authored filter as one row producer. */
pub struct PentaScored<
    S,
    A,
    B,
    C,
    D,
    E,
    EA,
    EB,
    EC,
    ED,
    EE,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    P3: CompileCondition<Plan: IndexedPlan>,
    P4: CompileCondition<Plan: IndexedPlan>,
    F,
> where
    BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>: Operator<S>,
    F: PentaFilter<S, A, B, C, D, E>,
{
    inner: BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>,
    filter: F,
    accepted: HandleMap<()>,
    /// Child notification scratch, reused across events.
    child_removed: Vec<RowHandle>,
    child_inserted: Vec<RowHandle>,
    marker: PhantomData<fn() -> (S, A, B, C, D, E)>,
}

impl<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F>
    PentaScored<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F>
where
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    P3: CompileCondition<Plan: IndexedPlan>,
    P4: CompileCondition<Plan: IndexedPlan>,
    BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>: Operator<S>,
    F: PentaFilter<S, A, B, C, D, E>,
{
    /// The penta tree with its authored filter applied at this boundary,
    /// whether the stream is scored here or continues into a `Chain`.
    pub(super) fn new(inner: BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>, filter: F) -> Self {
        Self {
            inner,
            filter,
            accepted: HandleMap::new(),
            child_removed: Vec::new(),
            child_inserted: Vec::new(),
            marker: PhantomData,
        }
    }
}

impl<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F> Operator<S>
    for PentaScored<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    D: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
    P1: CompileCondition<Plan: IndexedPlan> + 'static,
    P2: CompileCondition<Plan: IndexedPlan> + 'static,
    P3: CompileCondition<Plan: IndexedPlan> + 'static,
    P4: CompileCondition<Plan: IndexedPlan> + 'static,
    for<'a> BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>: Operator<
        S,
        View<'a> = Pair<
            Pair<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>, Leaf<'a, D>>,
            Leaf<'a, E>,
        >,
    >,
    F: PentaFilter<S, A, B, C, D, E> + 'static,
{
    type View<'a> = PentaRow<'a, S, EA, EB, EC, ED, EE, P1, P2, P3, P4>;
    type Evaluation =
        <BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4> as Operator<S>>::Evaluation;

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
        if F::ACCEPTS_ALL {
            self.inner.visit_evaluation(solution, evaluation, visitor);
            return;
        }
        self.inner
            .visit_evaluation(solution, evaluation, &mut |row| {
                let (a, b, c, d, e) = row.entities();
                let (ai, bi, ci, di, ei) = row.indexes();
                if self
                    .filter
                    .test(solution, a, b, c, d, e, ai, bi, ci, di, ei)
                {
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

    #[inline(always)]
    fn initialize(&mut self, solution: &S) {
        self.inner.initialize(solution);
        self.accepted.clear();
        self.child_removed.clear();
        self.child_inserted.clear();
        if F::ACCEPTS_ALL {
            return;
        }
        for handle in self.inner.handles() {
            let row = self
                .inner
                .resolve(solution, handle)
                .expect("live penta row");
            if self.test(solution, row) {
                self.accepted.insert(handle, ());
            }
        }
    }

    fn handles(&self) -> Vec<RowHandle> {
        if F::ACCEPTS_ALL {
            return self.inner.handles();
        }
        self.inner
            .handles()
            .into_iter()
            .filter(|h| self.accepted.get(*h).is_some())
            .collect()
    }

    #[inline]
    fn resolve<'a>(&'a self, solution: &'a S, handle: RowHandle) -> Option<Self::View<'a>> {
        if !F::ACCEPTS_ALL {
            self.accepted.get(handle)?;
        }
        self.inner.resolve(solution, handle)
    }

    fn visit_provenance(&self, handle: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        if F::ACCEPTS_ALL || self.accepted.get(handle).is_some() {
            self.inner.visit_provenance(handle, visitor);
        }
    }

    #[inline(always)]
    fn retract(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        removed: &mut Vec<RowHandle>,
        _inserted: &mut Vec<RowHandle>,
    ) {
        removed.clear();
        if F::ACCEPTS_ALL {
            self.inner.retract(
                solution,
                descriptor,
                index,
                removed,
                &mut self.child_inserted,
            );
            return;
        }
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

    #[inline(always)]
    fn insert(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        _removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        if F::ACCEPTS_ALL {
            inserted.clear();
            self.inner.insert(
                solution,
                descriptor,
                index,
                &mut self.child_removed,
                inserted,
            );
            return;
        }
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

impl<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F>
    PentaScored<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F>
where
    S: Send + Sync + 'static,
    P1: CompileCondition<Plan: IndexedPlan>,
    P2: CompileCondition<Plan: IndexedPlan>,
    P3: CompileCondition<Plan: IndexedPlan>,
    P4: CompileCondition<Plan: IndexedPlan>,
    BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>: Operator<S>,
    F: PentaFilter<S, A, B, C, D, E>,
{
    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn test<'a>(
        &self,
        solution: &S,
        row: <BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4> as Operator<S>>::View<'a>,
    ) -> bool
    where
        A: 'a,
        B: 'a,
        C: 'a,
        D: 'a,
        E: 'a,
        <BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4> as Operator<S>>::View<'a>:
            PentaEntities<'a, A, B, C, D, E>,
    {
        let (a, b, c, d, e) = row.entities();
        let (ai, bi, ci, di, ei) = row.indexes();
        self.filter
            .test(solution, a, b, c, d, e, ai, bi, ci, di, ei)
    }
}
