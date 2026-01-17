//! Construction heuristic phase implementation.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;
use tracing::{debug, info, trace};

use crate::heuristic::r#move::Move;
use crate::phase::construction::{ConstructionForager, EntityPlacer};
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

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
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M, P, Fo> ConstructionHeuristicPhase<S, M, P, Fo>
where
    S: PlanningSolution,
    M: Move<S>,
    P: EntityPlacer<S, M>,
    Fo: ConstructionForager<S, M>,
{
    /// Creates a new construction heuristic phase.
    pub fn new(placer: P, forager: Fo) -> Self {
        Self {
            placer,
            forager,
            _phantom: PhantomData,
        }
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

impl<S, D, M, P, Fo> Phase<S, D> for ConstructionHeuristicPhase<S, M, P, Fo>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    M: Move<S>,
    P: EntityPlacer<S, M>,
    Fo: ConstructionForager<S, M>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        let phase_start = Instant::now();
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        // Get all placements (entities that need values assigned)
        let placements = self.placer.get_placements(phase_scope.score_director());
        let total_entities = placements.len();

        info!(
            event = "construction_phase_start",
            entities_to_place = total_entities,
        );

        let mut entities_placed = 0;

        for mut placement in placements {
            // Check early termination
            if phase_scope.solver_scope().is_terminate_early() {
                debug!(
                    event = "construction_terminated_early",
                    entities_placed = entities_placed,
                    total_entities = total_entities,
                );
                break;
            }

            let step_start = Instant::now();
            let mut step_scope = StepScope::new(&mut phase_scope);
            let step_number = step_scope.phase_scope().step_count();

            let candidate_count = placement.move_count();
            trace!(
                event = "construction_step_start",
                step = step_number,
                candidates = candidate_count,
            );

            // Use forager to pick the best move index for this placement
            let selected_idx = self
                .forager
                .pick_move_index(&placement, step_scope.score_director_mut());

            if let Some(idx) = selected_idx {
                // Take ownership of the move
                let m = placement.take_move(idx);

                // Execute the move
                m.do_move(step_scope.score_director_mut());

                // Calculate and record the step score
                let step_score = step_scope.calculate_score();
                step_scope.set_step_score(step_score);

                entities_placed += 1;
                let step_duration = step_start.elapsed();

                debug!(
                    event = "construction_step_complete",
                    step = step_number,
                    candidates_evaluated = candidate_count,
                    selected_index = idx,
                    score = %step_score,
                    step_duration_ms = step_duration.as_millis() as u64,
                );
            } else {
                debug!(
                    event = "construction_no_move_selected",
                    step = step_number,
                    candidates = candidate_count,
                );
            }

            step_scope.complete();
        }

        // Update best solution at end of phase
        phase_scope.update_best_solution();

        let final_score = phase_scope.calculate_score();
        let phase_duration = phase_start.elapsed();

        info!(
            event = "construction_phase_end",
            entities_placed = entities_placed,
            total_entities = total_entities,
            final_score = %final_score,
            phase_duration_ms = phase_duration.as_millis() as u64,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "ConstructionHeuristic"
    }
}
