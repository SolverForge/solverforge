/// Construction heuristic phase that builds an initial solution.
///
/// This phase iterates over uninitialized entities and assigns values
/// to their planning variables using a greedy approach.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
/// * `P` - The entity placer type
/// * `Fo` - The forager type
pub struct ConstructionHeuristicPhase<S, M, P, Fo>
where
    S: PlanningSolution,
    M: Move<S>,
    P: EntityPlacer<S, M>,
    Fo: ConstructionForager<S, M>,
{
    placer: P,
    forager: Fo,
    construction_obligation: ConstructionObligation,
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M, P, Fo> ConstructionHeuristicPhase<S, M, P, Fo>
where
    S: PlanningSolution,
    M: Move<S>,
    P: EntityPlacer<S, M>,
    Fo: ConstructionForager<S, M>,
{
    pub fn new(placer: P, forager: Fo) -> Self {
        Self {
            placer,
            forager,
            construction_obligation: ConstructionObligation::default(),
            _phantom: PhantomData,
        }
    }

    pub fn with_construction_obligation(
        mut self,
        construction_obligation: ConstructionObligation,
    ) -> Self {
        self.construction_obligation = construction_obligation;
        self
    }
}

impl<S, M, P, Fo> Debug for ConstructionHeuristicPhase<S, M, P, Fo>
where
    S: PlanningSolution,
    M: Move<S>,
    P: EntityPlacer<S, M> + Debug,
    Fo: ConstructionForager<S, M> + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstructionHeuristicPhase")
            .field("placer", &self.placer)
            .field("forager", &self.forager)
            .finish()
    }
}

impl<S, D, BestCb, M, P, Fo> Phase<S, D, BestCb> for ConstructionHeuristicPhase<S, M, P, Fo>
where
    S: PlanningSolution,
    S::Score: Copy,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    P: EntityPlacer<S, M>,
    Fo: ConstructionForager<S, M>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
        let mut phase_scope =
            PhaseScope::with_phase_type(solver_scope, 0, "Construction Heuristic");
        let phase_index = phase_scope.phase_index();

        info!(
            event = "phase_start",
            phase = "Construction Heuristic",
            phase_index = phase_index,
        );

        let mut placement_cursor = self.placer.open_cursor(phase_scope.score_director());

        loop {
            if phase_scope
                .solver_scope_mut()
                .should_terminate_construction()
            {
                break;
            }

            let placement_generation_started = Instant::now();
            let mut placement_generation_interrupted = false;
            let next_placement = placement_cursor.next_placement(
                phase_scope.score_director(),
                |placement| placement_completed(placement, phase_scope.solver_scope()),
                || {
                    let should_stop = phase_scope.solver_scope().work_should_stop();
                    placement_generation_interrupted |= should_stop;
                    should_stop
                },
            );
            phase_scope.record_generation_time(placement_generation_started.elapsed());
            let Some(mut placement) = next_placement else {
                let terminated = phase_scope
                    .solver_scope_mut()
                    .should_terminate_construction();
                if placement_generation_interrupted && !terminated {
                    continue;
                }
                break;
            };

            let mut step_scope = StepScope::new(&mut phase_scope);

            // Time the whole streamed selection once. Per-candidate evaluation
            // time is tracked separately, so the remainder is cursor generation
            // and forager bookkeeping without a clock read for every tiny move.
            let evaluation_before = step_scope.phase_scope().stats().evaluation_time();
            let selection_started = Instant::now();
            let selection_result = self.forager.select_move_index(
                &mut placement,
                self.construction_obligation,
                &mut step_scope,
            );
            let selection_elapsed = selection_started.elapsed();
            let evaluation_elapsed = step_scope
                .phase_scope()
                .stats()
                .evaluation_time()
                .saturating_sub(evaluation_before);
            step_scope
                .phase_scope_mut()
                .record_generation_time(selection_elapsed.saturating_sub(evaluation_elapsed));

            // Use forager to pick the best move index for this placement.
            let selection = match selection_result {
                Some(selection) => selection,
                None => {
                    match settle_construction_interrupt(&mut step_scope) {
                        StepInterrupt::Restart => {
                            continue;
                        }
                        StepInterrupt::TerminatePhase => break,
                    }
                }
            };

            commit_selection(
                &mut placement,
                selection,
                self.construction_obligation,
                &mut step_scope,
            );

            step_scope.complete();
        }

        let previous_best_score = phase_scope.solver_scope().best_score().copied();

        // Update best solution at end of phase
        phase_scope.update_best_solution();
        if phase_scope.solver_scope().current_score() == previous_best_score.as_ref() {
            phase_scope.promote_current_solution_on_score_tie();
        }

        let best_score = phase_scope
            .solver_scope()
            .best_score()
            .map(|s| format!("{}", s))
            .unwrap_or_else(|| "none".to_string());

        let duration = phase_scope.elapsed();
        let steps = phase_scope.step_count();
        let speed = whole_units_per_second(steps, duration);
        let stats = phase_scope.stats();

        info!(
            event = "phase_end",
            phase = "Construction Heuristic",
            phase_index = phase_index,
            duration = %format_duration(duration),
            steps = steps,
            moves_generated = stats.moves_generated,
            moves_evaluated = stats.moves_evaluated,
            moves_accepted = stats.moves_accepted,
            score_calculations = stats.score_calculations,
            generation_time = %format_duration(stats.generation_time()),
            evaluation_time = %format_duration(stats.evaluation_time()),
            speed = speed,
            score = best_score,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "ConstructionHeuristic"
    }
}
