use std::any::TypeId;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use super::*;
use crate::builder::usize_element_source_key;
use crate::manager::SolverTerminalReason;
use crate::phase::Phase;
use crate::scope::SolverScope;

#[derive(Clone)]
struct Plan {
    elements: Vec<usize>,
    routes: Vec<Vec<usize>>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn phase() -> ListConstructionPhase<Plan, usize> {
    ListConstructionPhaseBuilder::<Plan, usize>::new(
        |plan| plan.elements.len(),
        |plan| plan.routes.iter().flatten().copied().collect(),
        |plan| plan.routes.len(),
        |plan, entity, element| plan.routes[entity].push(element),
        |plan, source_index| plan.elements[source_index],
        usize_element_source_key,
        0,
    )
    .create_phase()
}

#[test]
fn public_round_robin_observes_ordinary_limits() {
    let plan = Plan {
        elements: vec![0, 1, 2],
        routes: vec![Vec::new()],
        score: None,
    };
    let descriptor = SolutionDescriptor::new("Plan", TypeId::of::<Plan>());
    let director = ScoreDirector::simple(plan, descriptor, |plan, _| plan.routes.len());
    let mut scope = SolverScope::new(director);
    scope.start_solving();
    scope.inphase_step_count_limit = Some(0);
    let mut phase = phase();

    phase.solve(&mut scope);

    assert_eq!(scope.working_solution().routes[0].len(), 0);
    assert_eq!(
        scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
}
