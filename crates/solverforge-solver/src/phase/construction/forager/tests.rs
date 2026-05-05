// Tests for construction foragers.

use super::*;
use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::EntityReference;
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::{Director, ScoreDirector};
use std::any::TypeId;

#[derive(Clone, Debug)]
struct Queen {
    row: Option<i64>,
}

#[derive(Clone, Debug)]
struct NQueensSolution {
    queens: Vec<Queen>,
    score: Option<SoftScore>,
}

impl PlanningSolution for NQueensSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_queens(s: &NQueensSolution) -> &Vec<Queen> {
    &s.queens
}

fn get_queens_mut(s: &mut NQueensSolution) -> &mut Vec<Queen> {
    &mut s.queens
}

fn get_queen_row(s: &NQueensSolution, idx: usize, _variable_index: usize) -> Option<i64> {
    s.queens.get(idx).and_then(|q| q.row)
}

fn set_queen_row(s: &mut NQueensSolution, idx: usize, _variable_index: usize, v: Option<i64>) {
    if let Some(queen) = s.queens.get_mut(idx) {
        queen.row = v;
    }
}

fn create_test_director() -> ScoreDirector<NQueensSolution, ()> {
    let solution = NQueensSolution {
        queens: vec![Queen { row: None }],
        score: None,
    };

    let descriptor = create_descriptor();

    ScoreDirector::simple(solution, descriptor, |s, _| s.queens.len())
}

fn create_descriptor() -> SolutionDescriptor {
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Queen",
        "queens",
        get_queens,
        get_queens_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Queen", TypeId::of::<Queen>(), "queens").with_extractor(extractor);

    SolutionDescriptor::new("NQueensSolution", TypeId::of::<NQueensSolution>())
        .with_entity(entity_desc)
}

#[derive(Clone, Debug)]
struct ScoredDirector {
    working_solution: NQueensSolution,
    descriptor: SolutionDescriptor,
    unassigned_score: i64,
}

impl Director<NQueensSolution> for ScoredDirector {
    fn working_solution(&self) -> &NQueensSolution {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut NQueensSolution {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = SoftScore::of(
            self.working_solution
                .queens
                .iter()
                .map(|queen| queen.row.unwrap_or(self.unassigned_score))
                .sum(),
        );
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> NQueensSolution {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        (descriptor_index == 0).then_some(self.working_solution.queens.len())
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.queens.len())
    }

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
    }
}

type TestMove = ChangeMove<NQueensSolution, i64>;

fn create_placement() -> Placement<NQueensSolution, TestMove> {
    create_placement_with_values([1, 5, 3])
}

fn create_placement_with_values(
    values: impl IntoIterator<Item = i64>,
) -> Placement<NQueensSolution, TestMove> {
    let entity_ref = EntityReference::new(0, 0);
    let moves: Vec<TestMove> = values
        .into_iter()
        .map(|value| ChangeMove::new(0, Some(value), get_queen_row, set_queen_row, 0, "row", 0))
        .collect();
    Placement::new(entity_ref, moves)
}

fn create_scored_director(unassigned_score: i64) -> ScoredDirector {
    ScoredDirector {
        working_solution: NQueensSolution {
            queens: vec![Queen { row: None }],
            score: None,
        },
        descriptor: create_descriptor(),
        unassigned_score,
    }
}

#[test]
fn test_first_fit_forager() {
    let mut director = create_test_director();
    let mut placement = create_placement();

    let forager = FirstFitForager::<NQueensSolution, TestMove>::new();
    let selected = forager.pick_move_index(&placement, &mut director);

    // First Fit should pick the first move (index 0)
    assert_eq!(selected, ConstructionChoice::Select(0));

    // Take move and execute
    if let ConstructionChoice::Select(idx) = selected {
        let m = placement.moves.swap_remove(idx);
        m.do_move(&mut director);
    }
}

#[test]
fn test_best_fit_forager() {
    let mut director = create_test_director();
    let mut placement = create_placement();

    let forager = BestFitForager::<NQueensSolution, TestMove>::new();
    let selected = forager.pick_move_index(&placement, &mut director);

    // Best Fit picks some move when all score equally (empty constraint set)
    assert!(matches!(selected, ConstructionChoice::Select(_)));

    // Take move and execute
    if let ConstructionChoice::Select(idx) = selected {
        let m = placement.moves.swap_remove(idx);
        m.do_move(&mut director);
        let score = director.calculate_score();
        assert_eq!(score, SoftScore::of(0));
    }
}

#[test]
fn test_empty_placement() {
    let mut director = create_test_director();
    let placement: Placement<NQueensSolution, TestMove> =
        Placement::new(EntityReference::new(0, 0), vec![]);

    let forager = FirstFitForager::<NQueensSolution, TestMove>::new();
    let selected = forager.pick_move_index(&placement, &mut director);

    assert_eq!(selected, ConstructionChoice::KeepCurrent);
}

fn value_strength(m: &TestMove, _solution: &NQueensSolution) -> i64 {
    m.to_value().copied().unwrap_or(0)
}

#[test]
fn test_weakest_fit_forager() {
    let mut director = create_test_director();
    let mut placement = create_placement(); // values: 1, 5, 3

    let forager = WeakestFitForager::<NQueensSolution, TestMove>::new(value_strength);
    let selected = forager.pick_move_index(&placement, &mut director);

    // Weakest Fit should pick the move with lowest strength (index 0, value 1)
    assert_eq!(selected, ConstructionChoice::Select(0));

    if let ConstructionChoice::Select(idx) = selected {
        let m = placement.moves.swap_remove(idx);
        m.do_move(&mut director);
        let score = director.calculate_score();
        assert_eq!(score, SoftScore::of(0));
    }
}

#[test]
fn test_strongest_fit_forager() {
    let mut director = create_test_director();
    let mut placement = create_placement(); // values: 1, 5, 3

    let forager = StrongestFitForager::<NQueensSolution, TestMove>::new(value_strength);
    let selected = forager.pick_move_index(&placement, &mut director);

    // Strongest Fit should pick the move with highest strength (index 1, value 5)
    assert_eq!(selected, ConstructionChoice::Select(1));

    if let ConstructionChoice::Select(idx) = selected {
        let m = placement.moves.swap_remove(idx);
        m.do_move(&mut director);
        let score = director.calculate_score();
        assert_eq!(score, SoftScore::of(0));
    }
}

#[test]
fn weakest_fit_keeps_current_when_selected_move_does_not_beat_optional_baseline() {
    let mut director = create_scored_director(0);
    let placement = create_placement_with_values([0, 7, 3]).with_keep_current_legal(true);

    let forager = WeakestFitForager::<NQueensSolution, TestMove>::new(value_strength);

    assert_eq!(
        forager.pick_move_index(&placement, &mut director),
        ConstructionChoice::KeepCurrent
    );
}

#[test]
fn weakest_fit_selects_when_selected_move_beats_optional_baseline() {
    let mut director = create_scored_director(0);
    let placement = create_placement_with_values([1, 7, 3]).with_keep_current_legal(true);

    let forager = WeakestFitForager::<NQueensSolution, TestMove>::new(value_strength);

    assert_eq!(
        forager.pick_move_index(&placement, &mut director),
        ConstructionChoice::Select(0)
    );
}

#[test]
fn strongest_fit_keeps_current_when_selected_move_does_not_beat_optional_baseline() {
    let mut director = create_scored_director(0);
    let placement = create_placement_with_values([-5, -1, 0]).with_keep_current_legal(true);

    let forager = StrongestFitForager::<NQueensSolution, TestMove>::new(value_strength);

    assert_eq!(
        forager.pick_move_index(&placement, &mut director),
        ConstructionChoice::KeepCurrent
    );
}

#[test]
fn strongest_fit_selects_when_selected_move_beats_optional_baseline() {
    let mut director = create_scored_director(0);
    let placement = create_placement_with_values([-5, 7, 3]).with_keep_current_legal(true);

    let forager = StrongestFitForager::<NQueensSolution, TestMove>::new(value_strength);

    assert_eq!(
        forager.pick_move_index(&placement, &mut director),
        ConstructionChoice::Select(1)
    );
}

#[test]
fn first_fit_keeps_current_when_optional_baseline_is_not_beaten() {
    let mut director = create_scored_director(0);
    let placement = create_placement_with_values([-5, -1]).with_keep_current_legal(true);

    let forager = FirstFitForager::<NQueensSolution, TestMove>::new();

    assert_eq!(
        forager.pick_move_index(&placement, &mut director),
        ConstructionChoice::KeepCurrent
    );
}

#[test]
fn first_fit_selects_later_improving_candidate_when_earlier_one_is_worse() {
    let mut director = create_scored_director(0);
    let placement = create_placement_with_values([-5, 7, -1]).with_keep_current_legal(true);

    let forager = FirstFitForager::<NQueensSolution, TestMove>::new();

    assert_eq!(
        forager.pick_move_index(&placement, &mut director),
        ConstructionChoice::Select(1)
    );
}

#[test]
fn first_fit_selects_first_improving_candidate_over_optional_baseline() {
    let mut director = create_scored_director(0);
    let placement = create_placement_with_values([7, -5, 3]).with_keep_current_legal(true);

    let forager = FirstFitForager::<NQueensSolution, TestMove>::new();

    assert_eq!(
        forager.pick_move_index(&placement, &mut director),
        ConstructionChoice::Select(0)
    );
}

#[test]
fn best_fit_prefers_first_equal_score_candidate_over_keep_current() {
    let mut director = create_test_director();
    let placement = create_placement().with_keep_current_legal(true);

    let forager = BestFitForager::<NQueensSolution, TestMove>::new();

    assert_eq!(
        forager.pick_move_index(&placement, &mut director),
        ConstructionChoice::Select(0)
    );
}

#[test]
fn first_feasible_returns_keep_current_when_baseline_is_feasible() {
    let mut director = create_test_director();
    let placement = create_placement().with_keep_current_legal(true);

    let forager = FirstFeasibleForager::<NQueensSolution, TestMove>::new();

    assert_eq!(
        forager.pick_move_index(&placement, &mut director),
        ConstructionChoice::KeepCurrent
    );
}

#[test]
fn best_fit_keeps_current_when_all_candidates_are_worse_than_baseline() {
    let mut director = create_scored_director(0);
    let placement = create_placement_with_values([-5, -1]).with_keep_current_legal(true);

    let forager = BestFitForager::<NQueensSolution, TestMove>::new();

    assert_eq!(
        forager.pick_move_index(&placement, &mut director),
        ConstructionChoice::KeepCurrent
    );
}

#[test]
fn best_fit_selects_best_candidate_when_it_beats_baseline() {
    let mut director = create_scored_director(0);
    let placement = create_placement_with_values([-5, 7, 3]).with_keep_current_legal(true);

    let forager = BestFitForager::<NQueensSolution, TestMove>::new();

    assert_eq!(
        forager.pick_move_index(&placement, &mut director),
        ConstructionChoice::Select(1)
    );
}

#[test]
fn first_feasible_selects_first_feasible_candidate_when_baseline_is_infeasible() {
    let mut director = create_scored_director(-2);
    let placement = create_placement_with_values([-3, 1, 5]).with_keep_current_legal(true);

    let forager = FirstFeasibleForager::<NQueensSolution, TestMove>::new();

    assert_eq!(
        forager.pick_move_index(&placement, &mut director),
        ConstructionChoice::Select(1)
    );
}

#[test]
fn first_feasible_prefers_equal_score_candidate_over_infeasible_baseline() {
    let mut director = create_scored_director(-1);
    let placement = create_placement_with_values([-1, -2]).with_keep_current_legal(true);

    let forager = FirstFeasibleForager::<NQueensSolution, TestMove>::new();

    assert_eq!(
        forager.pick_move_index(&placement, &mut director),
        ConstructionChoice::Select(0)
    );
}
