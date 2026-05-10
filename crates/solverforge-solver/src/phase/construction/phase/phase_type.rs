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
    refresh_placements_each_step: bool,
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
            refresh_placements_each_step: false,
            construction_obligation: ConstructionObligation::default(),
            _phantom: PhantomData,
        }
    }

    pub fn with_live_placement_refresh(mut self) -> Self {
        self.refresh_placements_each_step = true;
        self
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
    M: Move<S> + 'static,
    P: EntityPlacer<S, M>,
    Fo: ConstructionForager<S, M> + 'static,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);
        let phase_index = phase_scope.phase_index();

        info!(
            event = "phase_start",
            phase = "Construction Heuristic",
            phase_index = phase_index,
        );

        let mut placements = if self.refresh_placements_each_step {
            None
        } else {
            let placement_generation_started = Instant::now();
            let placements = filter_completed_scalar_placements(
                self.placer.get_placements(phase_scope.score_director()),
                phase_scope.solver_scope(),
            );
            let placement_generation_elapsed = placement_generation_started.elapsed();
            let generated_moves = placements
                .iter()
                .map(|placement| u64::try_from(placement.moves.len()).unwrap_or(u64::MAX))
                .sum();
            phase_scope.record_generated_batch(generated_moves, placement_generation_elapsed);
            Some(placements.into_iter())
        };
        let mut pending_placement = None;

        loop {
            // Construction must complete — only stop for external flag or time limit,
            // never for step/move count limits (those are for local search).
            if phase_scope
                .solver_scope_mut()
                .should_terminate_construction()
            {
                break;
            }

            let mut placement = if self.refresh_placements_each_step {
                let placement_generation_started = Instant::now();
                let next_placement = self.placer.get_next_placement(
                    phase_scope.score_director(),
                    |placement| placement_completed(placement, phase_scope.solver_scope()),
                );
                let placement_generation_elapsed = placement_generation_started.elapsed();
                if let Some((placement, generated_moves)) = next_placement {
                    phase_scope
                        .record_generated_batch(generated_moves, placement_generation_elapsed);

                    placement
                } else {
                    break;
                }
            } else {
                match pending_placement
                    .take()
                    .or_else(|| placements.as_mut().and_then(Iterator::next))
                {
                    Some(placement) => placement,
                    None => break,
                }
            };

            let mut step_scope = StepScope::new(&mut phase_scope);

            // Use forager to pick the best move index for this placement
            let selection = match select_move_index(
                &self.forager,
                &placement,
                self.construction_obligation,
                &mut step_scope,
            ) {
                ConstructionSelection::Selected(selection) => selection,
                ConstructionSelection::Interrupted => {
                    match settle_construction_interrupt(&mut step_scope) {
                        StepInterrupt::Restart => {
                            if !self.refresh_placements_each_step {
                                pending_placement = Some(placement);
                            }
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
