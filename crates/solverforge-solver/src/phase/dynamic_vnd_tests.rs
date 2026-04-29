use std::any::TypeId;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use smallvec::smallvec;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::Director;

use super::*;
use crate::heuristic::r#move::metadata::{hash_str, MoveTabuScope};
use crate::heuristic::r#move::{Move, MoveTabuSignature};
use crate::heuristic::selector::move_selector::{
    ArenaMoveCursor, CandidateId, MoveCandidateRef, MoveCursor,
};
use crate::heuristic::selector::MoveSelector;
use crate::manager::SolverTerminalReason;
use crate::scope::SolverScope;

#[derive(Clone, Debug)]
struct InterruptPlan {
    value: i64,
    score: Option<SoftScore>,
}

impl PlanningSolution for InterruptPlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct InterruptDirector {
    working_solution: InterruptPlan,
    descriptor: SolutionDescriptor,
}

impl InterruptDirector {
    fn new() -> Self {
        Self {
            working_solution: InterruptPlan {
                value: 0,
                score: None,
            },
            descriptor: SolutionDescriptor::new("InterruptPlan", TypeId::of::<InterruptPlan>()),
        }
    }
}

impl Director<InterruptPlan> for InterruptDirector {
    fn working_solution(&self) -> &InterruptPlan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut InterruptPlan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = SoftScore::of(self.working_solution.value);
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> InterruptPlan {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, _descriptor_index: usize) -> Option<usize> {
        Some(0)
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(0)
    }

    fn constraint_metadata(&self) -> &[solverforge_scoring::ConstraintMetadata] {
        &[]
    }
}

#[derive(Clone, Debug)]
struct InterruptMove {
    doable: bool,
    score: i64,
}

impl Move<InterruptPlan> for InterruptMove {
    fn is_doable<D: Director<InterruptPlan>>(&self, _score_director: &D) -> bool {
        self.doable
    }

    fn do_move<D: Director<InterruptPlan>>(&self, score_director: &mut D) {
        score_director.working_solution_mut().value = self.score;
    }

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &[]
    }

    fn variable_name(&self) -> &str {
        "interrupt_move"
    }

    fn tabu_signature<D: Director<InterruptPlan>>(&self, _score_director: &D) -> MoveTabuSignature {
        MoveTabuSignature::new(
            MoveTabuScope::new(0, "interrupt_move"),
            smallvec![hash_str("dynamic_vnd_interrupt_move")],
            smallvec![hash_str("dynamic_vnd_interrupt_move")],
        )
    }
}

struct FlaggingCursor {
    inner: ArenaMoveCursor<InterruptPlan, InterruptMove>,
    terminate: Arc<AtomicBool>,
    trigger_index: usize,
}

impl MoveCursor<InterruptPlan, InterruptMove> for FlaggingCursor {
    fn next_candidate(&mut self) -> Option<CandidateId> {
        let next = self.inner.next_candidate();
        if let Some(index) = next {
            if index.index() == self.trigger_index {
                self.terminate.store(true, Ordering::SeqCst);
            }
        }
        next
    }

    fn candidate(
        &self,
        index: CandidateId,
    ) -> Option<MoveCandidateRef<'_, InterruptPlan, InterruptMove>> {
        self.inner.candidate(index)
    }

    fn take_candidate(&mut self, index: CandidateId) -> InterruptMove {
        self.inner.take_candidate(index)
    }
}

#[derive(Clone, Debug)]
struct FlaggingSelector {
    moves: Vec<InterruptMove>,
    terminate: Arc<AtomicBool>,
    trigger_index: usize,
}

impl MoveSelector<InterruptPlan, InterruptMove> for FlaggingSelector {
    type Cursor<'a>
        = FlaggingCursor
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<InterruptPlan>>(
        &'a self,
        _score_director: &D,
    ) -> Self::Cursor<'a> {
        FlaggingCursor {
            inner: ArenaMoveCursor::from_moves(self.moves.iter().cloned()),
            terminate: self.terminate.clone(),
            trigger_index: self.trigger_index,
        }
    }

    fn size<D: Director<InterruptPlan>>(&self, _score_director: &D) -> usize {
        self.moves.len()
    }
}

fn selector_with_non_doable_tail(terminate: Arc<AtomicBool>) -> FlaggingSelector {
    let tail_len = crate::phase::control::GENERATION_POLL_INTERVAL + 8;
    let mut moves = Vec::with_capacity(tail_len + 1);
    moves.push(InterruptMove {
        doable: true,
        score: 0,
    });
    moves.extend((0..tail_len).map(|_| InterruptMove {
        doable: false,
        score: 0,
    }));
    FlaggingSelector {
        moves,
        terminate,
        trigger_index: 1,
    }
}

#[test]
fn dynamic_vnd_polling_advances_through_non_doable_tail_after_cancel_request() {
    let terminate = Arc::new(AtomicBool::new(false));
    let director = InterruptDirector::new();
    let mut solver_scope = SolverScope::new(director).with_terminate(Some(terminate.as_ref()));
    solver_scope.start_solving();

    let mut phase = DynamicVndPhase::<InterruptPlan, InterruptMove, FlaggingSelector>::new(vec![
        selector_with_non_doable_tail(terminate.clone()),
    ]);
    phase.solve(&mut solver_scope);

    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::Cancelled
    );
}
