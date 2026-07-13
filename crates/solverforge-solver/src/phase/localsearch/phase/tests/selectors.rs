#[derive(Debug)]
struct NoopMove;

impl Move<TestSolution> for NoopMove {
    type Undo = ();

    fn is_doable<D: Director<TestSolution>>(&self, _score_director: &D) -> bool {
        true
    }

    fn do_move<D: Director<TestSolution>>(&self, _score_director: &mut D) -> Self::Undo {}

    fn undo_move<D: Director<TestSolution>>(&self, _score_director: &mut D, _undo: Self::Undo) {}

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &[]
    }

    fn variable_name(&self) -> &str {
        "noop"
    }

    fn tabu_signature<D: Director<TestSolution>>(
        &self,
        _score_director: &D,
    ) -> crate::heuristic::r#move::MoveTabuSignature {
        let scope = crate::heuristic::r#move::metadata::MoveTabuScope::new(0, "noop");
        let identity = crate::heuristic::r#move::metadata::hash_str("phase_tests_noop_move");
        crate::heuristic::r#move::MoveTabuSignature::new(
            scope,
            smallvec::smallvec![identity],
            smallvec::smallvec![identity],
        )
    }
}

#[derive(Debug)]
struct SlowOpenSelector(Duration);

impl MoveSelector<TestSolution, NoopMove> for SlowOpenSelector {
    type Cursor<'a>
        = ArenaMoveCursor<TestSolution, NoopMove>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<TestSolution>>(
        &'a self,
        _score_director: &D,
    ) -> Self::Cursor<'a> {
        thread::sleep(self.0);
        ArenaMoveCursor::from_moves(std::iter::once(NoopMove))
    }

    fn size<D: Director<TestSolution>>(&self, _score_director: &D) -> usize {
        1
    }
}

#[derive(Clone, Debug)]
struct ScoreFieldDirector {
    working_solution: TestSolution,
    descriptor: SolutionDescriptor,
}

impl ScoreFieldDirector {
    fn new() -> Self {
        Self {
            working_solution: TestSolution::with_score(SoftScore::of(0)),
            descriptor: create_minimal_descriptor(),
        }
    }
}

impl Director<TestSolution> for ScoreFieldDirector {
    fn working_solution(&self) -> &TestSolution {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut TestSolution {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        self.working_solution.score.unwrap_or(SoftScore::ZERO)
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> TestSolution {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        (descriptor_index == 0).then_some(1)
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(1)
    }

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
    }
}

#[derive(Clone, Copy, Debug)]
struct ScoreFieldMove(i64);

impl Move<TestSolution> for ScoreFieldMove {
    type Undo = Option<SoftScore>;

    fn is_doable<D: Director<TestSolution>>(&self, _score_director: &D) -> bool {
        true
    }

    fn do_move<D: Director<TestSolution>>(&self, score_director: &mut D) -> Self::Undo {
        let old_score = score_director.working_solution().score;
        score_director.before_variable_changed(0, 0);
        score_director.working_solution_mut().score = Some(SoftScore::of(self.0));
        score_director.after_variable_changed(0, 0);
        old_score
    }

    fn undo_move<D: Director<TestSolution>>(&self, score_director: &mut D, undo: Self::Undo) {
        score_director.before_variable_changed(0, 0);
        score_director.working_solution_mut().score = undo;
        score_director.after_variable_changed(0, 0);
    }

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &[0]
    }

    fn variable_name(&self) -> &str {
        "score"
    }

    fn tabu_signature<D: Director<TestSolution>>(
        &self,
        _score_director: &D,
    ) -> crate::heuristic::r#move::MoveTabuSignature {
        let scope = crate::heuristic::r#move::metadata::MoveTabuScope::new(0, "score");
        crate::heuristic::r#move::MoveTabuSignature::new(
            scope,
            smallvec::smallvec![self.0 as u64],
            smallvec::smallvec![self.0 as u64],
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct ScoreImprovementRequiredMove(i64);

impl Move<TestSolution> for ScoreImprovementRequiredMove {
    type Undo = <ScoreFieldMove as Move<TestSolution>>::Undo;

    fn is_doable<D: Director<TestSolution>>(&self, score_director: &D) -> bool {
        ScoreFieldMove(self.0).is_doable(score_director)
    }

    fn do_move<D: Director<TestSolution>>(&self, score_director: &mut D) -> Self::Undo {
        ScoreFieldMove(self.0).do_move(score_director)
    }

    fn undo_move<D: Director<TestSolution>>(&self, score_director: &mut D, undo: Self::Undo) {
        ScoreFieldMove(self.0).undo_move(score_director, undo);
    }

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &[0]
    }

    fn variable_name(&self) -> &str {
        "score"
    }

    fn requires_score_improvement(&self) -> bool {
        true
    }

    fn tabu_signature<D: Director<TestSolution>>(
        &self,
        score_director: &D,
    ) -> crate::heuristic::r#move::MoveTabuSignature {
        ScoreFieldMove(self.0).tabu_signature(score_director)
    }
}

#[derive(Debug)]
struct ScoreFieldSelector {
    scores: Vec<i64>,
}

impl ScoreFieldSelector {
    fn new(scores: impl Into<Vec<i64>>) -> Self {
        Self {
            scores: scores.into(),
        }
    }
}

#[derive(Debug)]
struct ContextSpySelector {
    saw_context: &'static AtomicBool,
    step_index: &'static AtomicU64,
    accepted_limit: &'static AtomicUsize,
}

impl MoveSelector<TestSolution, ScoreFieldMove> for ContextSpySelector {
    type Cursor<'a>
        = ArenaMoveCursor<TestSolution, ScoreFieldMove>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<TestSolution>>(
        &'a self,
        _score_director: &D,
    ) -> Self::Cursor<'a> {
        ArenaMoveCursor::from_moves([ScoreFieldMove(1)])
    }

    fn open_cursor_with_context<'a, D: Director<TestSolution>>(
        &'a self,
        _score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        self.saw_context.store(true, Ordering::SeqCst);
        self.step_index
            .store(context.step_index(), Ordering::SeqCst);
        self.accepted_limit.store(
            context.accepted_count_limit().unwrap_or(usize::MAX),
            Ordering::SeqCst,
        );
        ArenaMoveCursor::from_moves([ScoreFieldMove(1)])
    }

    fn size<D: Director<TestSolution>>(&self, _score_director: &D) -> usize {
        1
    }
}

impl MoveSelector<TestSolution, ScoreFieldMove> for ScoreFieldSelector {
    type Cursor<'a>
        = ArenaMoveCursor<TestSolution, ScoreFieldMove>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<TestSolution>>(
        &'a self,
        _score_director: &D,
    ) -> Self::Cursor<'a> {
        ArenaMoveCursor::from_moves(self.scores.iter().copied().map(ScoreFieldMove))
    }

    fn size<D: Director<TestSolution>>(&self, _score_director: &D) -> usize {
        self.scores.len()
    }
}

#[derive(Debug)]
struct ScoreImprovementRequiredSelector {
    scores: Vec<i64>,
}

impl ScoreImprovementRequiredSelector {
    fn new(scores: impl Into<Vec<i64>>) -> Self {
        Self {
            scores: scores.into(),
        }
    }
}

impl MoveSelector<TestSolution, ScoreImprovementRequiredMove> for ScoreImprovementRequiredSelector {
    type Cursor<'a>
        = ArenaMoveCursor<TestSolution, ScoreImprovementRequiredMove>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<TestSolution>>(
        &'a self,
        _score_director: &D,
    ) -> Self::Cursor<'a> {
        ArenaMoveCursor::from_moves(
            self.scores
                .iter()
                .copied()
                .map(ScoreImprovementRequiredMove),
        )
    }

    fn size<D: Director<TestSolution>>(&self, _score_director: &D) -> usize {
        self.scores.len()
    }
}

#[derive(Debug)]
struct AlwaysAcceptAcceptor;

impl Acceptor<TestSolution> for AlwaysAcceptAcceptor {
    fn is_accepted(
        &mut self,
        _last_step_score: &SoftScore,
        _move_score: &SoftScore,
        _move_signature: Option<&crate::heuristic::r#move::MoveTabuSignature>,
    ) -> bool {
        true
    }
}

#[derive(Debug)]
struct CancelOnDoableMove {
    score: i64,
    terminate: &'static AtomicBool,
}

impl Move<TestSolution> for CancelOnDoableMove {
    type Undo = Option<SoftScore>;

    fn is_doable<D: Director<TestSolution>>(&self, _score_director: &D) -> bool {
        self.terminate.store(true, Ordering::SeqCst);
        true
    }

    fn do_move<D: Director<TestSolution>>(&self, score_director: &mut D) -> Self::Undo {
        let old_score = score_director.working_solution().score;
        score_director.before_variable_changed(0, 0);
        score_director.working_solution_mut().score = Some(SoftScore::of(self.score));
        score_director.after_variable_changed(0, 0);
        old_score
    }

    fn undo_move<D: Director<TestSolution>>(&self, score_director: &mut D, undo: Self::Undo) {
        score_director.before_variable_changed(0, 0);
        score_director.working_solution_mut().score = undo;
        score_director.after_variable_changed(0, 0);
    }

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &[0]
    }

    fn variable_name(&self) -> &str {
        "score"
    }

    fn tabu_signature<D: Director<TestSolution>>(
        &self,
        _score_director: &D,
    ) -> crate::heuristic::r#move::MoveTabuSignature {
        let scope = crate::heuristic::r#move::metadata::MoveTabuScope::new(0, "score");
        crate::heuristic::r#move::MoveTabuSignature::new(
            scope,
            smallvec::smallvec![self.score as u64],
            smallvec::smallvec![self.score as u64],
        )
    }
}

#[derive(Debug)]
struct CancelOnDoableSelector {
    terminate: &'static AtomicBool,
}

impl MoveSelector<TestSolution, CancelOnDoableMove> for CancelOnDoableSelector {
    type Cursor<'a>
        = ArenaMoveCursor<TestSolution, CancelOnDoableMove>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<TestSolution>>(
        &'a self,
        _score_director: &D,
    ) -> Self::Cursor<'a> {
        ArenaMoveCursor::from_moves([
            CancelOnDoableMove {
                score: 1,
                terminate: self.terminate,
            },
            CancelOnDoableMove {
                score: 3,
                terminate: self.terminate,
            },
        ])
    }

    fn size<D: Director<TestSolution>>(&self, _score_director: &D) -> usize {
        2
    }
}
