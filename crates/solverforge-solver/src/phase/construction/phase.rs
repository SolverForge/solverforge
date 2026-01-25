//! Construction heuristic phase implementation.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use tracing::info;

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

impl<S, C, M, P, Fo> Phase<S, C> for ConstructionHeuristicPhase<S, M, P, Fo>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    M: Move<S>,
    P: EntityPlacer<S, M>,
    Fo: ConstructionForager<S, M>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, C>) {
        let phase_start = Instant::now();
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        info!(event = "phase_start", phase = "ConstructionHeuristic");

        // Get all placements (entities that need values assigned)
        let placements = self.placer.get_placements(phase_scope.score_director());

        let mut steps = 0u64;
        let mut last_score = None;

        for mut placement in placements {
            // Check early termination
            if phase_scope.solver_scope().is_terminate_early() {
                break;
            }

            let mut step_scope = StepScope::new(&mut phase_scope);

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
                last_score = Some(step_score);
                step_scope.set_step_score(step_score);
            }

            step_scope.complete();
            steps += 1;
        }

        // Update best solution at end of phase
        phase_scope.update_best_solution();

        let duration = phase_start.elapsed();
        let speed = if duration.as_secs_f64() > 0.0 {
            (steps as f64 / duration.as_secs_f64()) as u64
        } else {
            0
        };
        let final_score = last_score
            .or_else(|| phase_scope.solver_scope().best_score().copied())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        info!(
            event = "phase_end",
            phase = "ConstructionHeuristic",
            duration_ms = duration.as_millis() as u64,
            steps = steps,
            speed = speed,
            score = %final_score,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "ConstructionHeuristic"
    }
}
