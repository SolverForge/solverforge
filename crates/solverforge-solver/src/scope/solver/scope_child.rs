/* Child scopes: the configuration a partition's solver scope is built with,
the step/move/score-calculation budget children share, and the partition
bound that ends a partitioned phase. Included by `scope/solver.rs`. */

#[derive(Clone, Copy)]
pub(crate) struct SolverScopeChildConfig<'t, S: PlanningSolution> {
    terminate: Option<&'t AtomicBool>,
    runtime: Option<SolverRuntime<S>>,
    environment_mode: EnvironmentMode,
    time_deadline: Option<Instant>,
    phase_budget: Option<&'t PhaseBudget>,
    partition_bound: Option<&'t PartitionBound<'t>>,
    inphase_step_count_limit: Option<u64>,
    inphase_move_count_limit: Option<u64>,
    inphase_score_calc_count_limit: Option<u64>,
    inphase_best_score_limit: Option<S::Score>,
}

impl<'t, S: PlanningSolution> SolverScopeChildConfig<'t, S> {
    /// Child scopes built from this configuration also stop at `bound`.
    pub(crate) fn with_partition_bound(mut self, bound: Option<&'t PartitionBound<'t>>) -> Self {
        if bound.is_some() {
            self.partition_bound = bound;
        }
        self
    }

    pub(crate) fn build_scope<PD>(&self, score_director: PD, seed: u64) -> SolverScope<'t, S, PD>
    where
        PD: Director<S>,
    {
        let terminate = self
            .terminate
            .or_else(|| self.runtime.map(|runtime| runtime.cancel_flag()));
        let mut scope = SolverScope::new(score_director)
            .with_terminate(terminate)
            .with_runtime(self.runtime)
            .without_publication()
            .with_environment_mode(self.environment_mode)
            .with_seed(seed);
        scope.time_deadline = self.time_deadline;
        scope.phase_budget = self.phase_budget;
        scope.partition_bound = self.partition_bound;
        if self.phase_budget.is_none() {
            scope.inphase_step_count_limit = self.inphase_step_count_limit;
            scope.inphase_move_count_limit = self.inphase_move_count_limit;
            scope.inphase_score_calc_count_limit = self.inphase_score_calc_count_limit;
        }
        scope.inphase_best_score_limit = self.inphase_best_score_limit;
        scope
    }
}

/// The time and step count limits of a partitioned phase. The deadline is
/// the phase's, shared by all partitions; the step count limit applies to
/// each partition's solve, through the per-partition copies
/// [`Self::for_partition`] makes. Reaching one ends a partition's solve the
/// way a phase ends: it keeps its best solution for the merge, and the solve
/// is not terminated. A bound made inside a partition chains to the
/// enclosing one, so the enclosing partition's limits still count the inner
/// partitions' steps.
pub(crate) struct PartitionBound<'p> {
    parent: Option<&'p PartitionBound<'p>>,
    deadline: Option<Instant>,
    step_count_limit: Option<u64>,
    step_count: AtomicU64,
}

impl<'p> PartitionBound<'p> {
    /// The bound of a phase with termination `config`, starting now, chained
    /// to `parent`; `None` when `config` sets neither a time nor a step
    /// count limit. A time limit past what `Instant` represents never ends.
    pub(crate) fn new(
        parent: Option<&'p PartitionBound<'p>>,
        config: Option<&TerminationConfig>,
    ) -> Option<Self> {
        let config = config?;
        let time_limit = config.time_limit();
        if time_limit.is_none() && config.step_count_limit.is_none() {
            return None;
        }
        Some(Self {
            parent,
            deadline: time_limit.and_then(|limit| Instant::now().checked_add(limit)),
            step_count_limit: config.step_count_limit,
            step_count: AtomicU64::new(0),
        })
    }

    /// This bound for one partition: the same deadline, step count limit
    /// and enclosing bound, with the partition's own step count.
    pub(crate) fn for_partition(&self) -> Self {
        Self {
            parent: self.parent,
            deadline: self.deadline,
            step_count_limit: self.step_count_limit,
            step_count: AtomicU64::new(0),
        }
    }

    fn record_step(&self) {
        self.step_count.fetch_add(1, Ordering::SeqCst);
        if let Some(parent) = self.parent {
            parent.record_step();
        }
    }

    /// Whether a limit of this bound or an enclosing one is reached.
    pub(crate) fn reached(&self) -> bool {
        self.deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
            || self
                .step_count_limit
                .is_some_and(|limit| self.step_count.load(Ordering::SeqCst) >= limit)
            || self.parent.is_some_and(PartitionBound::reached)
    }
}

impl<'t, S: PlanningSolution, D: Director<S>, ProgressCb: ProgressCallback<S>>
    SolverScope<'t, S, D, ProgressCb>
{
    pub(crate) fn child_phase_budget(&self) -> PhaseBudget {
        PhaseBudget::from_scope(self)
    }

    pub(crate) fn child_config<'a>(
        &'a self,
        phase_budget: Option<&'a PhaseBudget>,
    ) -> SolverScopeChildConfig<'a, S> {
        let phase_budget = self
            .phase_budget
            .or_else(|| phase_budget.filter(|budget| budget.has_limits()));
        SolverScopeChildConfig {
            terminate: self.terminate,
            runtime: self.runtime,
            environment_mode: self.environment_mode,
            time_deadline: self.child_time_deadline(),
            phase_budget,
            partition_bound: self.partition_bound,
            inphase_step_count_limit: self.inphase_step_count_limit,
            inphase_move_count_limit: self.inphase_move_count_limit,
            inphase_score_calc_count_limit: self.inphase_score_calc_count_limit,
            inphase_best_score_limit: self.inphase_best_score_limit,
        }
    }

    /// The bound of the partitioned phase this scope solves a partition of.
    pub(crate) fn partition_bound(&self) -> Option<&'t PartitionBound<'t>> {
        self.partition_bound
    }

    fn partition_bound_reached(&self) -> bool {
        self.partition_bound.is_some_and(PartitionBound::reached)
    }
}
