include!("support/control.rs");

fn create_placer(
    values: Vec<i64>,
) -> QueuedEntityPlacer<
    NQueensSolution,
    i64,
    FromSolutionEntitySelector,
    StaticValueSelector<NQueensSolution, i64>,
> {
    let es = FromSolutionEntitySelector::new(0);
    let vs = StaticValueSelector::new(values);
    QueuedEntityPlacer::new(es, vs, get_queen_row, set_queen_row, 0, 0, "row")
}

#[derive(Clone, Debug)]
struct ConstructionPauseEntity {
    value: Option<i64>,
}

#[derive(Clone, Debug)]
struct ConstructionPauseSolution {
    entities: Vec<ConstructionPauseEntity>,
    score: Option<SoftScore>,
    eval_gate: Option<BlockingEvaluationGate>,
    solvable_mode: ConstructionPauseSolvableMode,
}

#[derive(Clone, Copy, Debug)]
enum ConstructionPauseSolvableMode {
    FirstFitMax64,
    FirstFitDecisive,
    BestFitKeepCurrent,
}

impl ConstructionPauseSolution {
    fn new(eval_gate: Option<BlockingEvaluationGate>) -> Self {
        Self::with_entity_count(1, eval_gate)
    }

    fn with_entity_count(entity_count: usize, eval_gate: Option<BlockingEvaluationGate>) -> Self {
        Self {
            entities: vec![ConstructionPauseEntity { value: None }; entity_count],
            score: None,
            eval_gate,
            solvable_mode: ConstructionPauseSolvableMode::FirstFitMax64,
        }
    }

    fn keep_current_pause(eval_gate: Option<BlockingEvaluationGate>) -> Self {
        Self {
            solvable_mode: ConstructionPauseSolvableMode::BestFitKeepCurrent,
            ..Self::new(eval_gate)
        }
    }

    fn decisive(eval_gate: BlockingEvaluationGate) -> Self {
        Self {
            eval_gate: Some(eval_gate),
            solvable_mode: ConstructionPauseSolvableMode::FirstFitDecisive,
            ..Self::new(None)
        }
    }
}

impl PlanningSolution for ConstructionPauseSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct ConstructionPauseDirector {
    working_solution: ConstructionPauseSolution,
    descriptor: SolutionDescriptor,
    score_mode: ConstructionPauseScoreMode,
}

#[derive(Clone, Copy, Debug)]
enum ConstructionPauseScoreMode {
    AssignedSum {
        unassigned_score: i64,
    },
    CompletionBonus {
        incomplete_score: i64,
        complete_score: i64,
    },
}

impl ConstructionPauseDirector {
    fn new(solution: ConstructionPauseSolution) -> Self {
        Self::with_score_mode(
            solution,
            ConstructionPauseScoreMode::AssignedSum {
                unassigned_score: 0,
            },
        )
    }

    fn with_score_mode(
        solution: ConstructionPauseSolution,
        score_mode: ConstructionPauseScoreMode,
    ) -> Self {
        Self {
            working_solution: solution,
            descriptor: SolutionDescriptor::new(
                "ConstructionPauseSolution",
                TypeId::of::<ConstructionPauseSolution>(),
            ),
            score_mode,
        }
    }
}

impl Director<ConstructionPauseSolution> for ConstructionPauseDirector {
    fn working_solution(&self) -> &ConstructionPauseSolution {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut ConstructionPauseSolution {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = match self.score_mode {
            ConstructionPauseScoreMode::AssignedSum { unassigned_score } => SoftScore::of(
                self.working_solution
                    .entities
                    .iter()
                    .map(|entity| entity.value.unwrap_or(unassigned_score))
                    .sum(),
            ),
            ConstructionPauseScoreMode::CompletionBonus {
                incomplete_score,
                complete_score,
            } => {
                let all_assigned = self
                    .working_solution
                    .entities
                    .iter()
                    .all(|entity| entity.value.is_some());
                SoftScore::of(if all_assigned {
                    complete_score
                } else {
                    incomplete_score
                })
            }
        };
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> ConstructionPauseSolution {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        (descriptor_index == 0).then_some(self.working_solution.entities.len())
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.entities.len())
    }

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
    }
}

#[derive(Clone, Debug)]
struct ConstructionPauseMove {
    entity_index: usize,
    entity_indices: [usize; 1],
    value: i64,
    doable: bool,
    eval_gate: Option<BlockingEvaluationGate>,
}

impl ConstructionPauseMove {
    fn new(
        entity_index: usize,
        value: i64,
        doable: bool,
        eval_gate: Option<BlockingEvaluationGate>,
    ) -> Self {
        Self {
            entity_index,
            entity_indices: [entity_index],
            value,
            doable,
            eval_gate,
        }
    }
}

impl Move<ConstructionPauseSolution> for ConstructionPauseMove {
    type Undo = Option<i64>;

    fn is_doable<D: Director<ConstructionPauseSolution>>(&self, _score_director: &D) -> bool {
        if let Some(gate) = &self.eval_gate {
            gate.on_evaluation();
        }
        self.doable
    }

    fn do_move<D: Director<ConstructionPauseSolution>>(&self, score_director: &mut D) -> Self::Undo {
        let old_value = score_director.working_solution().entities[self.entity_index].value;
        score_director.working_solution_mut().entities[self.entity_index].value = Some(self.value);
        old_value
    }

    fn undo_move<D: Director<ConstructionPauseSolution>>(
        &self,
        score_director: &mut D,
        undo: Self::Undo,
    ) {
        score_director.working_solution_mut().entities[self.entity_index].value = undo;
    }

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        "value"
    }

    fn tabu_signature<D: Director<ConstructionPauseSolution>>(
        &self,
        _score_director: &D,
    ) -> crate::heuristic::r#move::MoveTabuSignature {
        let scope = crate::heuristic::r#move::metadata::MoveTabuScope::new(0, "value");
        crate::heuristic::r#move::MoveTabuSignature::new(
            scope,
            smallvec::smallvec![
                crate::heuristic::r#move::metadata::hash_str("construction_pause_move"),
                self.entity_index as u64,
                self.value as u64,
            ],
            smallvec::smallvec![
                crate::heuristic::r#move::metadata::hash_str("construction_pause_move"),
                self.entity_index as u64,
                self.value as u64,
            ],
        )
    }
}

#[derive(Clone, Debug)]
struct ConstructionPausePlacer {
    eval_gate: Option<BlockingEvaluationGate>,
}

impl ConstructionPausePlacer {
    fn new(eval_gate: Option<BlockingEvaluationGate>) -> Self {
        Self { eval_gate }
    }
}

struct ConstructionTestPlacerCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    placements: std::vec::IntoIter<Placement<S, M>>,
}

impl<S, M> EntityPlacerCursor<S, M> for ConstructionTestPlacerCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    type CandidateCursor = ArenaMoveCursor<S, M>;

    fn next_placement<D, IsCompleted, ShouldStop>(
        &mut self,
        _score_director: &D,
        mut is_completed: IsCompleted,
        mut should_stop: ShouldStop,
    ) -> Option<Placement<S, M>>
    where
        D: Director<S>,
        IsCompleted: FnMut(&Placement<S, M>) -> bool,
        ShouldStop: FnMut() -> bool,
    {
        while !should_stop() {
            let placement = self.placements.next()?;
            if !is_completed(&placement) {
                return Some(placement);
            }
        }
        None
    }
}

impl EntityPlacer<ConstructionPauseSolution, ConstructionPauseMove> for ConstructionPausePlacer {
    type Cursor<'a>
        = ConstructionTestPlacerCursor<ConstructionPauseSolution, ConstructionPauseMove>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<ConstructionPauseSolution>>(
        &'a self,
        score_director: &D,
    ) -> Self::Cursor<'a> {
        let placements = score_director
            .working_solution()
            .entities
            .iter()
            .enumerate()
            .filter_map(|(entity_index, entity)| {
                if entity.value.is_some() {
                    return None;
                }

                let moves: Vec<_> = (0..65)
                    .map(|value| {
                        ConstructionPauseMove::new(
                            entity_index,
                            value as i64,
                            value == 64,
                            (value == 0).then(|| self.eval_gate.clone()).flatten(),
                        )
                    })
                    .collect();

                Some(Placement::new(
                    EntityReference::new(0, entity_index),
                    ArenaMoveCursor::from_moves(moves),
                ))
            })
            .collect::<Vec<_>>()
            .into_iter();
        ConstructionTestPlacerCursor { placements }
    }
}

#[derive(Clone, Debug)]
struct ScoredConstructionPlacer {
    values: Vec<i64>,
    keep_current_legal: bool,
    eval_gate: Option<BlockingEvaluationGate>,
}

impl ScoredConstructionPlacer {
    fn new(values: Vec<i64>, keep_current_legal: bool) -> Self {
        Self {
            values,
            keep_current_legal,
            eval_gate: None,
        }
    }

    fn with_eval_gate(mut self, eval_gate: Option<BlockingEvaluationGate>) -> Self {
        self.eval_gate = eval_gate;
        self
    }
}

impl EntityPlacer<ConstructionPauseSolution, ConstructionPauseMove> for ScoredConstructionPlacer {
    type Cursor<'a>
        = ConstructionTestPlacerCursor<ConstructionPauseSolution, ConstructionPauseMove>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<ConstructionPauseSolution>>(
        &'a self,
        score_director: &D,
    ) -> Self::Cursor<'a> {
        let placements = score_director
            .working_solution()
            .entities
            .iter()
            .enumerate()
            .filter_map(|(entity_index, entity)| {
                if entity.value.is_some() {
                    return None;
                }

                let moves: Vec<_> = self
                    .values
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(idx, value)| {
                        ConstructionPauseMove::new(
                            entity_index,
                            value,
                            true,
                            (idx == 0).then(|| self.eval_gate.clone()).flatten(),
                        )
                    })
                    .collect();

                Some(
                    Placement::new(
                        EntityReference::new(0, entity_index),
                        ArenaMoveCursor::from_moves(moves),
                    )
                        .with_keep_current_legal(self.keep_current_legal),
                )
            })
            .collect::<Vec<_>>()
            .into_iter();
        ConstructionTestPlacerCursor { placements }
    }
}

impl Solvable for ConstructionPauseSolution {
    fn solve(
        self,
        runtime: SolverRuntime<Self>,
        _provenance: Option<crate::stats::QualifiedCandidateTraceRunProvenance>,
    ) {
        let eval_gate = self.eval_gate.clone();
        let solvable_mode = self.solvable_mode;
        let mut solver_scope = SolverScope::new_with_callback(
            ConstructionPauseDirector::new(self),
            (),
            None,
            Some(runtime),
        );

        solver_scope.start_solving();

        match solvable_mode {
            ConstructionPauseSolvableMode::FirstFitMax64 => {
                let mut phase = ConstructionHeuristicPhase::new(
                    ConstructionPausePlacer::new(eval_gate),
                    FirstFitForager::new(),
                );
                phase.solve(&mut solver_scope);
            }
            ConstructionPauseSolvableMode::FirstFitDecisive => {
                let mut phase = ConstructionHeuristicPhase::new(
                    ScoredConstructionPlacer::new((0..66).collect(), false)
                        .with_eval_gate(eval_gate),
                    FirstFitForager::new(),
                );
                phase.solve(&mut solver_scope);
            }
            ConstructionPauseSolvableMode::BestFitKeepCurrent => {
                let mut phase = ConstructionHeuristicPhase::new(
                    ScoredConstructionPlacer::new(vec![-5], true).with_eval_gate(eval_gate),
                    BestFitForager::new(),
                );
                phase.solve(&mut solver_scope);
            }
        }

        let mut current_score = solver_scope.current_score().copied();
        let best_score = if let Some(best_score) = solver_scope.best_score().copied() {
            best_score
        } else {
            let score = solver_scope.calculate_score();
            current_score.get_or_insert(score);
            score
        };

        let telemetry = solver_scope.stats().snapshot();
        let solution = solver_scope.score_director().clone_working_solution();

        if runtime.is_cancel_requested() {
            runtime.emit_cancelled(current_score, Some(best_score), telemetry);
        } else {
            runtime.emit_completed(
                solution,
                current_score,
                best_score,
                telemetry,
                SolverTerminalReason::Completed,
            );
        }
    }
}
