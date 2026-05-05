use super::*;
use std::any::TypeId;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use smallvec::smallvec;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{HardSoftScore, SoftScore};
use solverforge_scoring::Director;

use crate::heuristic::r#move::metadata::{hash_str, MoveTabuScope};
use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::r#move::{Move, MoveTabuSignature};
use crate::heuristic::selector::move_selector::{
    ArenaMoveCursor, CandidateId, MoveCandidateRef, MoveCursor,
};
use crate::heuristic::selector::ChangeMoveSelector;
use crate::manager::SolverTerminalReason;
use crate::scope::SolverScope;
use crate::test_utils::{create_nqueens_director, get_queen_row, set_queen_row, NQueensSolution};

type NQueensMove = ChangeMove<NQueensSolution, i64>;

fn create_move_selector(
    values: Vec<i64>,
) -> ChangeMoveSelector<
    NQueensSolution,
    i64,
    crate::heuristic::selector::FromSolutionEntitySelector,
    crate::heuristic::selector::StaticValueSelector<NQueensSolution, i64>,
> {
    ChangeMoveSelector::simple(get_queen_row, set_queen_row, 0, 0, "row", values)
}

#[test]
fn test_vnd_improves_solution() {
    let director = create_nqueens_director(&[0, 0, 0, 0]);
    let mut solver_scope = SolverScope::new(director);

    let initial_score = solver_scope.calculate_score();

    let values: Vec<i64> = (0..4).collect();
    let mut phase: VndPhase<_, NQueensMove> = VndPhase::new((
        create_move_selector(values.clone()),
        create_move_selector(values),
    ));

    phase.solve(&mut solver_scope);

    let final_score = solver_scope.best_score().copied().unwrap_or(initial_score);
    assert!(final_score >= initial_score);
}

#[test]
fn test_vnd_single_neighborhood() {
    let director = create_nqueens_director(&[0, 0, 0, 0]);
    let mut solver_scope = SolverScope::new(director);

    let initial_score = solver_scope.calculate_score();

    let values: Vec<i64> = (0..4).collect();
    let mut phase: VndPhase<_, NQueensMove> = VndPhase::new((create_move_selector(values),));

    phase.solve(&mut solver_scope);

    let final_score = solver_scope.best_score().copied().unwrap_or(initial_score);
    assert!(final_score >= initial_score);
}

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

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
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
            smallvec![hash_str("vnd_interrupt_move")],
            smallvec![hash_str("vnd_interrupt_move")],
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
fn tuple_vnd_polling_advances_through_non_doable_tail_after_cancel_request() {
    let terminate = Arc::new(AtomicBool::new(false));
    let director = InterruptDirector::new();
    let mut solver_scope = SolverScope::new(director).with_terminate(Some(terminate.as_ref()));
    solver_scope.start_solving();

    let mut phase = VndPhase::<_, InterruptMove>::new((
        selector_with_non_doable_tail(terminate.clone()),
        selector_with_non_doable_tail(terminate.clone()),
    ));
    phase.solve(&mut solver_scope);

    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::Cancelled
    );
}

#[derive(Clone, Debug)]
struct HardRepairPlan {
    hard: i64,
    soft: i64,
    score: Option<HardSoftScore>,
}

impl PlanningSolution for HardRepairPlan {
    type Score = HardSoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct HardRepairDirector {
    working_solution: HardRepairPlan,
    descriptor: SolutionDescriptor,
}

impl HardRepairDirector {
    fn new() -> Self {
        Self {
            working_solution: HardRepairPlan {
                hard: -1,
                soft: 0,
                score: None,
            },
            descriptor: SolutionDescriptor::new("HardRepairPlan", TypeId::of::<HardRepairPlan>()),
        }
    }
}

impl Director<HardRepairPlan> for HardRepairDirector {
    fn working_solution(&self) -> &HardRepairPlan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut HardRepairPlan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> HardSoftScore {
        let score = HardSoftScore::of(self.working_solution.hard, self.working_solution.soft);
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> HardRepairPlan {
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

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
    }
}

#[derive(Clone, Debug)]
struct HardRepairMove {
    hard: i64,
    soft: i64,
    require_hard: bool,
}

impl Move<HardRepairPlan> for HardRepairMove {
    fn is_doable<D: Director<HardRepairPlan>>(&self, _score_director: &D) -> bool {
        true
    }

    fn do_move<D: Director<HardRepairPlan>>(&self, score_director: &mut D) {
        let previous_hard = score_director.working_solution().hard;
        let previous_soft = score_director.working_solution().soft;
        let solution = score_director.working_solution_mut();
        solution.hard = self.hard;
        solution.soft = self.soft;
        score_director.register_undo(Box::new(move |solution: &mut HardRepairPlan| {
            solution.hard = previous_hard;
            solution.soft = previous_soft;
        }));
    }

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &[]
    }

    fn variable_name(&self) -> &str {
        "hard_repair_move"
    }

    fn requires_hard_improvement(&self) -> bool {
        self.require_hard
    }

    fn tabu_signature<D: Director<HardRepairPlan>>(
        &self,
        _score_director: &D,
    ) -> MoveTabuSignature {
        MoveTabuSignature::new(
            MoveTabuScope::new(0, "hard_repair_move"),
            smallvec![hash_str("tuple_vnd_hard_repair_move")],
            smallvec![hash_str("tuple_vnd_hard_repair_move")],
        )
    }
}

#[derive(Clone, Debug)]
struct HardRepairSelector {
    moves: Vec<HardRepairMove>,
}

impl MoveSelector<HardRepairPlan, HardRepairMove> for HardRepairSelector {
    type Cursor<'a>
        = ArenaMoveCursor<HardRepairPlan, HardRepairMove>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<HardRepairPlan>>(
        &'a self,
        _score_director: &D,
    ) -> Self::Cursor<'a> {
        ArenaMoveCursor::from_moves(self.moves.iter().cloned())
    }

    fn size<D: Director<HardRepairPlan>>(&self, _score_director: &D) -> usize {
        self.moves.len()
    }
}

#[test]
fn tuple_vnd_rejects_hard_neutral_repair_move_when_hard_improvement_required() {
    let director = HardRepairDirector::new();
    let mut solver_scope = SolverScope::new(director);
    let mut phase = VndPhase::<_, HardRepairMove>::new((HardRepairSelector {
        moves: vec![HardRepairMove {
            hard: -1,
            soft: 10,
            require_hard: true,
        }],
    },));

    phase.solve(&mut solver_scope);

    let solution = solver_scope.working_solution();
    assert_eq!(solution.hard, -1);
    assert_eq!(solution.soft, 0);
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(-1, 0))
    );
    let stats = solver_scope.stats();
    assert_eq!(stats.moves_generated, 1);
    assert_eq!(stats.moves_evaluated, 1);
    assert_eq!(stats.moves_acceptor_rejected, 1);
    assert_eq!(stats.moves_hard_neutral, 1);
}
