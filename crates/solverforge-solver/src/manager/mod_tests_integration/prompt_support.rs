use std::any::TypeId;
use std::time::Duration;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::Director;

use super::super::{Solvable, SolverRuntime, SolverTerminalReason};
use super::gates::{BlockingEvaluationGate, BlockingPoint};
use crate::heuristic::r#move::Move;
use crate::heuristic::selector::MoveSelector;
use crate::phase::localsearch::{BestScoreForager, HillClimbingAcceptor, LocalSearchPhase};
use crate::phase::Phase;
use crate::scope::SolverScope;

#[derive(Clone, Debug)]
pub(super) struct PromptControlSolution {
    value: i64,
    score: Option<SoftScore>,
    selector: PromptControlSelector,
    time_limit: Option<Duration>,
}

impl PromptControlSolution {
    pub(super) fn generation_blocked(
        total_moves: usize,
        block_at: usize,
        blocker: BlockingPoint,
        time_limit: Option<Duration>,
    ) -> Self {
        Self {
            value: 0,
            score: None,
            selector: PromptControlSelector::Generation(BlockingGenerationSelector {
                total_moves,
                block_at,
                blocker,
            }),
            time_limit,
        }
    }

    pub(super) fn evaluation_blocked(total_moves: usize, gate: BlockingEvaluationGate) -> Self {
        Self {
            value: 0,
            score: None,
            selector: PromptControlSelector::Evaluation(BlockingEvaluationSelector {
                total_moves,
                gate,
            }),
            time_limit: None,
        }
    }
}

impl PlanningSolution for PromptControlSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct PromptControlDirector {
    working_solution: PromptControlSolution,
    descriptor: SolutionDescriptor,
}

impl PromptControlDirector {
    fn new(solution: PromptControlSolution) -> Self {
        Self {
            working_solution: solution,
            descriptor: SolutionDescriptor::new(
                "PromptControlSolution",
                TypeId::of::<PromptControlSolution>(),
            ),
        }
    }
}

impl Director<PromptControlSolution> for PromptControlDirector {
    fn working_solution(&self) -> &PromptControlSolution {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut PromptControlSolution {
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

    fn clone_working_solution(&self) -> PromptControlSolution {
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
}

#[derive(Clone, Debug)]
struct NoOpMove {
    eval_gate: Option<BlockingEvaluationGate>,
}

impl NoOpMove {
    fn new(eval_gate: Option<BlockingEvaluationGate>) -> Self {
        Self { eval_gate }
    }
}

impl Move<PromptControlSolution> for NoOpMove {
    fn is_doable<D: Director<PromptControlSolution>>(&self, _score_director: &D) -> bool {
        true
    }

    fn do_move<D: Director<PromptControlSolution>>(&self, _score_director: &mut D) {
        if let Some(gate) = &self.eval_gate {
            gate.on_evaluation();
        }
    }

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &[]
    }

    fn variable_name(&self) -> &str {
        "noop"
    }

    fn tabu_signature<D: Director<PromptControlSolution>>(
        &self,
        _score_director: &D,
    ) -> crate::heuristic::r#move::MoveTabuSignature {
        let scope = crate::heuristic::r#move::metadata::MoveTabuScope::new(0, "noop");
        let identity = crate::heuristic::r#move::metadata::hash_str("prompt_support_noop_move");
        crate::heuristic::r#move::MoveTabuSignature::new(
            scope,
            smallvec::smallvec![identity],
            smallvec::smallvec![identity],
        )
    }
}

#[derive(Clone, Debug)]
struct BlockingGenerationSelector {
    total_moves: usize,
    block_at: usize,
    blocker: BlockingPoint,
}

#[derive(Clone, Debug)]
struct BlockingEvaluationSelector {
    total_moves: usize,
    gate: BlockingEvaluationGate,
}

#[derive(Clone, Debug)]
enum PromptControlSelector {
    Generation(BlockingGenerationSelector),
    Evaluation(BlockingEvaluationSelector),
}

impl MoveSelector<PromptControlSolution, NoOpMove> for BlockingGenerationSelector {
    fn open_cursor<'a, D: Director<PromptControlSolution>>(
        &'a self,
        _score_director: &D,
    ) -> impl Iterator<Item = NoOpMove> + 'a {
        (0..self.total_moves).map(move |index| {
            if index == self.block_at {
                self.blocker.block();
            }
            NoOpMove::new(None)
        })
    }

    fn size<D: Director<PromptControlSolution>>(&self, _score_director: &D) -> usize {
        self.total_moves
    }
}

impl MoveSelector<PromptControlSolution, NoOpMove> for BlockingEvaluationSelector {
    fn open_cursor<'a, D: Director<PromptControlSolution>>(
        &'a self,
        _score_director: &D,
    ) -> impl Iterator<Item = NoOpMove> + 'a {
        (0..self.total_moves).map(move |_| NoOpMove::new(Some(self.gate.clone())))
    }

    fn size<D: Director<PromptControlSolution>>(&self, _score_director: &D) -> usize {
        self.total_moves
    }
}

impl MoveSelector<PromptControlSolution, NoOpMove> for PromptControlSelector {
    fn open_cursor<'a, D: Director<PromptControlSolution>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = NoOpMove> + 'a {
        enum PromptControlIter<A, B> {
            Generation(A),
            Evaluation(B),
        }

        impl<T, A, B> Iterator for PromptControlIter<A, B>
        where
            A: Iterator<Item = T>,
            B: Iterator<Item = T>,
        {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Self::Generation(iter) => iter.next(),
                    Self::Evaluation(iter) => iter.next(),
                }
            }
        }

        match self {
            Self::Generation(selector) => {
                PromptControlIter::Generation(selector.open_cursor(score_director))
            }
            Self::Evaluation(selector) => {
                PromptControlIter::Evaluation(selector.open_cursor(score_director))
            }
        }
    }

    fn size<D: Director<PromptControlSolution>>(&self, score_director: &D) -> usize {
        match self {
            Self::Generation(selector) => selector.size(score_director),
            Self::Evaluation(selector) => selector.size(score_director),
        }
    }
}

impl Solvable for PromptControlSolution {
    fn solve(self, runtime: SolverRuntime<Self>) {
        let mut solver_scope = SolverScope::new_with_callback(
            PromptControlDirector::new(self),
            (),
            None,
            Some(runtime),
        );

        solver_scope.start_solving();
        if let Some(time_limit) = solver_scope.working_solution().time_limit {
            solver_scope.set_time_limit(time_limit);
        }
        let score = solver_scope.calculate_score();
        let solution = solver_scope.score_director().clone_working_solution();
        solver_scope.set_best_solution(solution, score);
        runtime.emit_best_solution(
            solver_scope.score_director().clone_working_solution(),
            Some(score),
            score,
            solver_scope.stats().snapshot(),
        );

        let selector = solver_scope.working_solution().selector.clone();
        let mut phase = LocalSearchPhase::new(
            selector,
            HillClimbingAcceptor::new(),
            BestScoreForager::new(),
            Some(1),
        );
        phase.solve(&mut solver_scope);

        let terminal_reason = solver_scope.terminal_reason();
        let telemetry = solver_scope.stats().snapshot();
        let current_score = solver_scope.current_score().copied();
        let best_score = solver_scope.best_score().copied().unwrap_or(score);

        match terminal_reason {
            SolverTerminalReason::Cancelled => {
                runtime.emit_cancelled(current_score, Some(best_score), telemetry);
            }
            reason => runtime.emit_completed(
                solver_scope.score_director().clone_working_solution(),
                current_score,
                best_score,
                telemetry,
                reason,
            ),
        }
    }
}
