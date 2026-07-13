#[derive(Clone, Debug)]
struct PullCountingPlacer {
    values: Vec<i64>,
    keep_current_legal: bool,
    pulls: Arc<AtomicUsize>,
}

struct PullCountingPlacerCursor<'a> {
    placer: &'a PullCountingPlacer,
    emitted: bool,
}

struct PullCountingCandidateCursor {
    values: std::vec::IntoIter<i64>,
    pulls: Arc<AtomicUsize>,
    store: CandidateStore<ConstructionPauseSolution, ConstructionPauseMove>,
}

impl MoveCursor<ConstructionPauseSolution, ConstructionPauseMove>
    for PullCountingCandidateCursor
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        let value = self.values.next()?;
        self.pulls.fetch_add(1, Ordering::SeqCst);
        Some(
            self.store
                .push(ConstructionPauseMove::new(0, value, true, None)),
        )
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, ConstructionPauseSolution, ConstructionPauseMove>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ConstructionPauseMove {
        self.store.take_candidate(id)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
    }
}

impl EntityPlacerCursor<ConstructionPauseSolution, ConstructionPauseMove>
    for PullCountingPlacerCursor<'_>
{
    type CandidateCursor = PullCountingCandidateCursor;

    fn next_placement<D, IsCompleted, ShouldStop>(
        &mut self,
        score_director: &D,
        mut is_completed: IsCompleted,
        mut should_stop: ShouldStop,
    ) -> Option<Placement<ConstructionPauseSolution, ConstructionPauseMove, Self::CandidateCursor>>
    where
        D: Director<ConstructionPauseSolution>,
        IsCompleted: FnMut(
            &Placement<ConstructionPauseSolution, ConstructionPauseMove, Self::CandidateCursor>,
        ) -> bool,
        ShouldStop: FnMut() -> bool,
    {
        if self.emitted
            || should_stop()
            || score_director.working_solution().entities[0]
                .value
                .is_some()
        {
            return None;
        }
        self.emitted = true;
        let placement = Placement::new(
            EntityReference::new(0, 0),
            PullCountingCandidateCursor {
                values: self.placer.values.clone().into_iter(),
                pulls: Arc::clone(&self.placer.pulls),
                store: CandidateStore::new(),
            },
        )
        .with_keep_current_legal(self.placer.keep_current_legal);
        (!is_completed(&placement)).then_some(placement)
    }
}

impl EntityPlacer<ConstructionPauseSolution, ConstructionPauseMove> for PullCountingPlacer {
    type Cursor<'a>
        = PullCountingPlacerCursor<'a>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<ConstructionPauseSolution>>(
        &'a self,
        _score_director: &D,
    ) -> Self::Cursor<'a> {
        PullCountingPlacerCursor {
            placer: self,
            emitted: false,
        }
    }
}

#[test]
fn first_fit_stops_after_the_decisive_candidate() {
    let pulls = Arc::new(AtomicUsize::new(0));
    let placer = PullCountingPlacer {
        values: (0..66).collect(),
        keep_current_legal: false,
        pulls: Arc::clone(&pulls),
    };
    let director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    let mut phase = ConstructionHeuristicPhase::new(placer, FirstFitForager::new());

    phase.solve(&mut solver_scope);

    assert_eq!(pulls.load(Ordering::SeqCst), 1);
    assert_eq!(solver_scope.working_solution().entities[0].value, Some(0));
    assert_eq!(solver_scope.stats().moves_generated, 1);
    assert_eq!(solver_scope.stats().moves_evaluated, 1);
    assert_eq!(solver_scope.stats().moves_accepted, 1);
    assert_eq!(solver_scope.stats().moves_applied, 1);
    assert_eq!(solver_scope.stats().score_calculations, 1);
}

#[test]
fn first_feasible_stops_after_the_decisive_candidate() {
    let pulls = Arc::new(AtomicUsize::new(0));
    let placer = PullCountingPlacer {
        values: (0..66).collect(),
        keep_current_legal: true,
        pulls: Arc::clone(&pulls),
    };
    let director = ConstructionPauseDirector::with_score_mode(
        ConstructionPauseSolution::new(None),
        ConstructionPauseScoreMode::AssignedSum {
            unassigned_score: -2,
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    let mut phase = ConstructionHeuristicPhase::new(placer, FirstFeasibleForager::new());

    phase.solve(&mut solver_scope);

    assert_eq!(pulls.load(Ordering::SeqCst), 1);
    assert_eq!(solver_scope.working_solution().entities[0].value, Some(0));
    assert_eq!(solver_scope.stats().moves_generated, 1);
    assert_eq!(solver_scope.stats().moves_evaluated, 1);
    assert_eq!(solver_scope.stats().moves_accepted, 1);
    assert_eq!(solver_scope.stats().moves_applied, 1);
    assert_eq!(solver_scope.stats().score_calculations, 2);
}
