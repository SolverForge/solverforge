use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{Director, RecordingDirector};

use crate::heuristic::r#move::{Move, MoveArena};
use crate::heuristic::selector::MoveSelector;
use crate::phase::control::{
    append_interruptibly, settle_search_interrupt, should_interrupt_evaluation, StepInterrupt,
};
use crate::phase::Phase;
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepScope};

pub struct DynamicVndPhase<S, M, MS> {
    neighborhoods: Vec<MS>,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, MS> DynamicVndPhase<S, M, MS> {
    pub fn new(neighborhoods: Vec<MS>) -> Self {
        Self {
            neighborhoods,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, MS: Debug> Debug for DynamicVndPhase<S, M, MS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicVndPhase")
            .field("neighborhoods", &self.neighborhoods)
            .finish()
    }
}

impl<S, D, ProgressCb, M, MS> Phase<S, D, ProgressCb> for DynamicVndPhase<S, M, MS>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    M: Move<S>,
    MS: MoveSelector<S, M>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        let mut arena = MoveArena::<M>::new();
        let mut phase_scope = PhaseScope::new(solver_scope, 0);
        let mut current_score = phase_scope.calculate_score();
        let mut k = 0usize;

        while k < self.neighborhoods.len() {
            let mut step_scope = StepScope::new(&mut phase_scope);
            arena.reset();
            let generation_interrupted = {
                let iter = self.neighborhoods[k].iter_moves(step_scope.score_director());
                append_interruptibly(&step_scope, &mut arena, iter)
            };
            if generation_interrupted {
                match settle_search_interrupt(&mut step_scope) {
                    StepInterrupt::Restart => continue,
                    StepInterrupt::TerminatePhase => break,
                }
            }

            match find_best_improving_move_index(&arena, &mut step_scope, &current_score) {
                MoveSearchResult::Found(selected_index, selected_score) => {
                    let selected_move = arena.take(selected_index);
                    selected_move.do_move(step_scope.score_director_mut());
                    step_scope.set_step_score(selected_score);
                    current_score = selected_score;
                    step_scope.phase_scope_mut().update_best_solution();
                    step_scope.complete();
                    k = 0;
                }
                MoveSearchResult::NotFound => {
                    step_scope.complete();
                    k += 1;
                }
                MoveSearchResult::Interrupted => match settle_search_interrupt(&mut step_scope) {
                    StepInterrupt::Restart => continue,
                    StepInterrupt::TerminatePhase => break,
                },
            }
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "VariableNeighborhoodDescent"
    }
}

enum MoveSearchResult<Sc> {
    Found(usize, Sc),
    NotFound,
    Interrupted,
}

fn find_best_improving_move_index<S, D, ProgressCb, M>(
    arena: &MoveArena<M>,
    step_scope: &mut StepScope<'_, '_, '_, S, D, ProgressCb>,
    current_score: &S::Score,
) -> MoveSearchResult<S::Score>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    M: Move<S>,
{
    let mut best: Option<(usize, S::Score)> = None;

    for i in 0..arena.len() {
        if should_interrupt_evaluation(step_scope, i) {
            return MoveSearchResult::Interrupted;
        }

        let m = arena.get(i).unwrap();

        if !m.is_doable(step_scope.score_director()) {
            continue;
        }

        let mut recording = RecordingDirector::new(step_scope.score_director_mut());
        m.do_move(&mut recording);
        let move_score = recording.calculate_score();
        recording.undo_changes();

        if move_score > *current_score {
            match &best {
                Some((_, best_score)) if move_score > *best_score => {
                    best = Some((i, move_score));
                }
                None => best = Some((i, move_score)),
                _ => {}
            }
        }
    }

    match best {
        Some((index, score)) => MoveSearchResult::Found(index, score),
        None => MoveSearchResult::NotFound,
    }
}
