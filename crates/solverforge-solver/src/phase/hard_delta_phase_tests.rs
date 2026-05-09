use std::any::TypeId;

use smallvec::smallvec;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::BendableScore;
use solverforge_scoring::Director;

use crate::heuristic::r#move::metadata::{hash_str, MoveTabuScope};
use crate::heuristic::r#move::{Move, MoveTabuSignature};
use crate::heuristic::selector::move_selector::ArenaMoveCursor;
use crate::heuristic::selector::MoveSelector;
use crate::phase::localsearch::VndPhase;
use crate::phase::traits::Phase;
use crate::scope::SolverScope;

#[test]
fn vnd_accepts_required_repair_that_improves_later_hard_level() {
    let director = BendableRepairDirector::new();
    let mut solver_scope = SolverScope::new(director);
    let mut phase = VndPhase::<BendableRepairPlan, BendableRepairMove, BendableRepairSelector>::new(
        vec![BendableRepairSelector {
            moves: vec![BendableRepairMove {
                first_hard: 0,
                second_hard: -5,
                soft: -100,
                require_hard: true,
            }],
        }],
        None,
    );

    phase.solve(&mut solver_scope);

    let solution = solver_scope.working_solution();
    assert_eq!(solution.first_hard, 0);
    assert_eq!(solution.second_hard, -5);
    assert_eq!(solution.soft, -100);
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(BendableScore::of([0, -5], [-100]))
    );
    let stats = solver_scope.stats();
    assert_eq!(stats.moves_accepted, 1);
    assert_eq!(stats.moves_acceptor_rejected, 1);
    assert_eq!(stats.moves_hard_improving, 1);
    assert_eq!(stats.moves_hard_neutral, 1);
}

#[derive(Clone, Debug)]
struct BendableRepairPlan {
    first_hard: i64,
    second_hard: i64,
    soft: i64,
    score: Option<BendableScore<2, 1>>,
}

impl PlanningSolution for BendableRepairPlan {
    type Score = BendableScore<2, 1>;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct BendableRepairDirector {
    working_solution: BendableRepairPlan,
    descriptor: SolutionDescriptor,
}

impl BendableRepairDirector {
    fn new() -> Self {
        Self {
            working_solution: BendableRepairPlan {
                first_hard: 0,
                second_hard: -10,
                soft: 0,
                score: None,
            },
            descriptor: SolutionDescriptor::new(
                "BendableRepairPlan",
                TypeId::of::<BendableRepairPlan>(),
            ),
        }
    }
}

impl Director<BendableRepairPlan> for BendableRepairDirector {
    fn working_solution(&self) -> &BendableRepairPlan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut BendableRepairPlan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> BendableScore<2, 1> {
        let score = BendableScore::of(
            [
                self.working_solution.first_hard,
                self.working_solution.second_hard,
            ],
            [self.working_solution.soft],
        );
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> BendableRepairPlan {
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
struct BendableRepairMove {
    first_hard: i64,
    second_hard: i64,
    soft: i64,
    require_hard: bool,
}

impl Move<BendableRepairPlan> for BendableRepairMove {
    fn is_doable<D: Director<BendableRepairPlan>>(&self, _score_director: &D) -> bool {
        true
    }

    fn do_move<D: Director<BendableRepairPlan>>(&self, score_director: &mut D) {
        let previous_first_hard = score_director.working_solution().first_hard;
        let previous_second_hard = score_director.working_solution().second_hard;
        let previous_soft = score_director.working_solution().soft;
        let previous_score = score_director.working_solution().score;
        let solution = score_director.working_solution_mut();
        solution.first_hard = self.first_hard;
        solution.second_hard = self.second_hard;
        solution.soft = self.soft;
        score_director.register_undo(Box::new(move |solution: &mut BendableRepairPlan| {
            solution.first_hard = previous_first_hard;
            solution.second_hard = previous_second_hard;
            solution.soft = previous_soft;
            solution.score = previous_score;
        }));
    }

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &[]
    }

    fn variable_name(&self) -> &str {
        "bendable_repair_move"
    }

    fn requires_hard_improvement(&self) -> bool {
        self.require_hard
    }

    fn tabu_signature<D: Director<BendableRepairPlan>>(
        &self,
        _score_director: &D,
    ) -> MoveTabuSignature {
        MoveTabuSignature::new(
            MoveTabuScope::new(0, "bendable_repair_move"),
            smallvec![hash_str("bendable_repair_move")],
            smallvec![hash_str("bendable_repair_move")],
        )
    }
}

#[derive(Clone, Debug)]
struct BendableRepairSelector {
    moves: Vec<BendableRepairMove>,
}

impl MoveSelector<BendableRepairPlan, BendableRepairMove> for BendableRepairSelector {
    type Cursor<'a>
        = ArenaMoveCursor<BendableRepairPlan, BendableRepairMove>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<BendableRepairPlan>>(
        &'a self,
        _score_director: &D,
    ) -> Self::Cursor<'a> {
        ArenaMoveCursor::from_moves(self.moves.iter().cloned())
    }

    fn size<D: Director<BendableRepairPlan>>(&self, _score_director: &D) -> usize {
        self.moves.len()
    }
}
