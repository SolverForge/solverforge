//! Integration tests for LocalSearchPhase to debug score stagnation.
//!
//! These tests verify that score improves during local search when
//! improving moves are available.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SimpleScore;
use solverforge_core::ConstraintRef;
use solverforge_scoring::api::constraint_set::{ConstraintSet, IncrementalConstraint};
use solverforge_scoring::{ScoreDirector, ShadowVariableSupport};

use crate::heuristic::r#move::Move;
use crate::phase::localsearch::{AcceptedCountForager, HillClimbingAcceptor, LocalSearchPhase};
use crate::phase::Phase;
use crate::scope::SolverScope;

// =============================================================================
// Test Domain - Simplified Employee Scheduling
// =============================================================================

/// Simplified solution mimicking employee scheduling.
/// - 10 shifts (entities)
/// - 5 employees (values 0-4)
/// - Constraint: penalize unassigned shifts
#[derive(Clone, Debug)]
struct TestSolution {
    /// Shift assignments: shift_id -> Option<employee_id>
    shifts: Vec<Option<i32>>,
    score: Option<SimpleScore>,
}

impl PlanningSolution for TestSolution {
    type Score = SimpleScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl ShadowVariableSupport for TestSolution {
    fn update_element_shadow(&mut self, _entity_idx: usize, _position: usize, _element_idx: usize) {
        // No shadow variables
    }

    fn retract_element_shadow(
        &mut self,
        _entity_idx: usize,
        _position: usize,
        _element_idx: usize,
    ) {
        // No shadow variables
    }

    fn update_all_shadows(&mut self) {
        // No shadow variables
    }
}

impl TestSolution {
    fn new(num_shifts: usize) -> Self {
        Self {
            shifts: vec![None; num_shifts],
            score: None,
        }
    }
}

// =============================================================================
// Test Constraint - Penalize unassigned shifts
// =============================================================================

struct UnassignedPenaltyConstraint;

impl IncrementalConstraint<TestSolution, SimpleScore> for UnassignedPenaltyConstraint {
    fn evaluate(&self, solution: &TestSolution) -> SimpleScore {
        let unassigned = solution.shifts.iter().filter(|s| s.is_none()).count() as i64;
        SimpleScore::of(-unassigned)
    }

    fn match_count(&self, solution: &TestSolution) -> usize {
        solution.shifts.iter().filter(|s| s.is_none()).count()
    }

    fn initialize(&mut self, solution: &TestSolution) -> SimpleScore {
        self.evaluate(solution)
    }

    fn on_insert(
        &mut self,
        solution: &TestSolution,
        _descriptor_index: usize,
        entity_index: usize,
    ) -> SimpleScore {
        // If shift is now unassigned, add penalty
        if solution
            .shifts
            .get(entity_index)
            .copied()
            .flatten()
            .is_none()
        {
            SimpleScore::of(-1)
        } else {
            SimpleScore::of(0)
        }
    }

    fn on_retract(
        &mut self,
        solution: &TestSolution,
        _descriptor_index: usize,
        entity_index: usize,
    ) -> SimpleScore {
        // If shift was unassigned, remove penalty
        if solution
            .shifts
            .get(entity_index)
            .copied()
            .flatten()
            .is_none()
        {
            SimpleScore::of(1)
        } else {
            SimpleScore::of(0)
        }
    }

    fn reset(&mut self) {}

    fn name(&self) -> &str {
        "UnassignedPenalty"
    }

    fn constraint_ref(&self) -> ConstraintRef {
        ConstraintRef::new("", "UnassignedPenalty")
    }
}

// =============================================================================
// Test Move - ChangeMove for shift assignment
// =============================================================================

/// Simple move that assigns an employee to a shift.
#[derive(Debug)]
struct TestChangeMove {
    entity_index: usize,
    to_value: Option<i32>,
}

impl TestChangeMove {
    fn new(entity_index: usize, to_value: Option<i32>) -> Self {
        Self {
            entity_index,
            to_value,
        }
    }
}

impl Move<TestSolution> for TestChangeMove {
    fn is_doable<C>(&self, score_director: &ScoreDirector<TestSolution, C>) -> bool
    where
        C: ConstraintSet<TestSolution, SimpleScore>,
    {
        let current = score_director
            .working_solution()
            .shifts
            .get(self.entity_index)
            .copied()
            .flatten();
        current != self.to_value
    }

    fn do_move<C>(&self, score_director: &mut ScoreDirector<TestSolution, C>)
    where
        C: ConstraintSet<TestSolution, SimpleScore>,
    {
        let old_value = score_director
            .working_solution()
            .shifts
            .get(self.entity_index)
            .copied()
            .flatten();

        score_director.before_variable_changed(0, self.entity_index);
        score_director.working_solution_mut().shifts[self.entity_index] = self.to_value;
        score_director.after_variable_changed(0, self.entity_index);

        let idx = self.entity_index;
        score_director.register_undo(Box::new(move |s: &mut TestSolution| {
            s.shifts[idx] = old_value;
        }));
    }

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        std::slice::from_ref(&self.entity_index)
    }

    fn variable_name(&self) -> &str {
        "employee"
    }
}

// =============================================================================
// Test Move Selector - Generates all change moves
// =============================================================================

struct TestMoveSelector {
    num_shifts: usize,
    employees: Vec<i32>,
}

impl TestMoveSelector {
    fn new(num_shifts: usize, employees: Vec<i32>) -> Self {
        Self {
            num_shifts,
            employees,
        }
    }
}

impl Debug for TestMoveSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestMoveSelector").finish()
    }
}

impl crate::heuristic::selector::MoveSelector<TestSolution, TestChangeMove> for TestMoveSelector {
    fn iter_moves<'a, C>(
        &'a self,
        _score_director: &'a ScoreDirector<TestSolution, C>,
    ) -> Box<dyn Iterator<Item = TestChangeMove> + 'a>
    where
        C: ConstraintSet<TestSolution, SimpleScore>,
    {
        let iter = (0..self.num_shifts)
            .flat_map(move |shift| self.employees.iter().map(move |&emp| (shift, emp)))
            .map(|(shift, emp)| TestChangeMove::new(shift, Some(emp)));
        Box::new(iter)
    }

    fn size<C>(&self, _score_director: &ScoreDirector<TestSolution, C>) -> usize
    where
        C: ConstraintSet<TestSolution, SimpleScore>,
    {
        self.num_shifts * self.employees.len()
    }
}

// =============================================================================
// Tests
// =============================================================================

/// Test that LocalSearchPhase improves score when improving moves exist.
///
/// This test simulates the employee-scheduling scenario:
/// - Start with all shifts unassigned (score = -10)
/// - Run local search with hill climbing
/// - Verify score improves as shifts get assigned
#[test]
fn test_local_search_score_improves() {
    // Setup solution with 10 unassigned shifts
    let solution = TestSolution::new(10);
    let constraints = (UnassignedPenaltyConstraint,);

    let mut director = ScoreDirector::new(solution, constraints);

    // Verify initial score is -10 (all unassigned)
    let initial_score = director.calculate_score();
    assert_eq!(initial_score, SimpleScore::of(-10));

    // Create local search phase
    let move_selector = TestMoveSelector::new(10, vec![0, 1, 2, 3, 4]);
    let acceptor = HillClimbingAcceptor::new();
    let forager = AcceptedCountForager::new(100);

    let mut phase = LocalSearchPhase::new(move_selector, acceptor, forager, Some(10), 100);

    // Create solver scope
    let mut solver_scope = SolverScope::new(director);

    // Run phase
    phase.solve(&mut solver_scope);

    // Verify score improved
    let final_score = solver_scope.calculate_score();
    eprintln!(
        "Initial score: {:?}, Final score: {:?}",
        initial_score, final_score
    );
    assert!(
        final_score > initial_score,
        "Score should improve! Initial: {:?}, Final: {:?}",
        initial_score,
        final_score
    );
}

/// Test move evaluation and undo cycle.
///
/// Verifies that:
/// 1. Move changes the score
/// 2. Undo restores the original score
#[test]
fn test_move_evaluation_undo_cycle() {
    let solution = TestSolution::new(5);
    let constraints = (UnassignedPenaltyConstraint,);

    let mut director = ScoreDirector::new(solution, constraints);

    let initial_score = director.calculate_score();
    assert_eq!(initial_score, SimpleScore::of(-5));

    // Create a move that assigns employee 0 to shift 0
    let test_move = TestChangeMove::new(0, Some(0));

    // Verify move is doable
    assert!(test_move.is_doable(&director));

    // Save score snapshot before move
    director.save_score_snapshot();

    // Execute move
    test_move.do_move(&mut director);

    // Calculate score after move - should be -4 (one less unassigned)
    let post_move_score = director.calculate_score();
    assert_eq!(
        post_move_score,
        SimpleScore::of(-4),
        "Score after move should be -4"
    );

    // Undo the move
    director.undo_changes();

    // Calculate score after undo - should be back to -5
    let restored_score = director.calculate_score();
    assert_eq!(
        restored_score, initial_score,
        "Score after undo should match initial"
    );

    // Verify solution state is restored
    assert!(
        director.working_solution().shifts[0].is_none(),
        "Shift 0 should be unassigned after undo"
    );
}

/// Test that multiple move evaluations don't accumulate score drift.
#[test]
fn test_multiple_move_evaluations_no_drift() {
    let solution = TestSolution::new(5);
    let constraints = (UnassignedPenaltyConstraint,);

    let mut director = ScoreDirector::new(solution, constraints);

    let initial_score = director.calculate_score();
    assert_eq!(initial_score, SimpleScore::of(-5));

    // Simulate the move evaluation loop from LocalSearchPhase
    for entity_idx in 0..5 {
        for employee_id in 0..3 {
            let test_move = TestChangeMove::new(entity_idx, Some(employee_id));

            if test_move.is_doable(&director) {
                director.save_score_snapshot();
                test_move.do_move(&mut director);
                let _move_score = director.calculate_score();
                director.undo_changes();
            }
        }
    }

    // Verify no score drift occurred
    let final_score = director.calculate_score();
    assert_eq!(
        final_score, initial_score,
        "Score should be unchanged after evaluating many moves"
    );

    // Verify solution state is unchanged
    for (i, shift) in director.working_solution().shifts.iter().enumerate() {
        assert!(shift.is_none(), "Shift {} should still be unassigned", i);
    }
}

/// Test that accepted move is actually applied (not just evaluated).
#[test]
fn test_accepted_move_application() {
    let solution = TestSolution::new(3);
    let constraints = (UnassignedPenaltyConstraint,);

    let mut director = ScoreDirector::new(solution, constraints);

    let initial_score = director.calculate_score();
    assert_eq!(initial_score, SimpleScore::of(-3));

    // Execute a move for real (not just evaluation)
    let test_move = TestChangeMove::new(0, Some(0));

    // This simulates what LocalSearchPhase does when a move is accepted
    test_move.do_move(&mut director);
    director.clear_undo_stack(); // Commit the move

    // Verify move was applied
    let final_score = director.calculate_score();
    assert_eq!(
        final_score,
        SimpleScore::of(-2),
        "Score should be -2 after applying one assignment"
    );
    assert_eq!(
        director.working_solution().shifts[0],
        Some(0),
        "Shift 0 should be assigned to employee 0"
    );
}

/// Test that shuffled selection order provides uniform entity coverage.
///
/// This test verifies that when selection order is Shuffled (the default),
/// all entities get visited roughly equally over multiple iterations,
/// unlike the original sequential order where early entities are visited
/// much more frequently when foragers quit early.
#[test]
fn test_shuffled_selection_order_uniform_coverage() {
    use crate::heuristic::r#move::MoveImpl;
    use crate::heuristic::selector::{
        BasicVariableFnPtrs, MoveSelector, MoveSelectorImpl, SelectionOrder,
    };

    // Create a solution with many entities to demonstrate the coverage difference
    let num_entities = 20;
    let solution = TestSolution::new(num_entities);
    let constraints = (UnassignedPenaltyConstraint,);
    let director = ScoreDirector::new(solution, constraints);

    // Track which entities get visited in the first N moves
    let moves_to_sample = 10; // Sample only first 10 moves per iteration
    let iterations = 100;

    // Create shuffled selector using the actual MoveSelectorImpl
    let fn_ptrs = BasicVariableFnPtrs {
        entity_count: |s: &TestSolution| s.shifts.len(),
        value_range: |_: &TestSolution| vec![0i32, 1, 2, 3, 4],
        getter: |s: &TestSolution, idx: usize| s.shifts.get(idx).copied().flatten(),
        setter: |_s: &mut TestSolution, _idx: usize, _val: Option<i32>| {},
        variable_name: "employee",
        descriptor_index: 0,
    };

    // Test with Shuffled order (default)
    let shuffled_selector: MoveSelectorImpl<TestSolution, i32> =
        MoveSelectorImpl::change_with_order(fn_ptrs, SelectionOrder::Shuffled);

    // Count how many times each entity index appears in the first N moves
    let mut entity_visit_counts = vec![0usize; num_entities];

    for _ in 0..iterations {
        let moves: Vec<_> = shuffled_selector
            .iter_moves(&director)
            .take(moves_to_sample)
            .collect();
        for m in moves {
            if let MoveImpl::Change(change_move) = m {
                let entity_idx = change_move.entity_indices()[0];
                entity_visit_counts[entity_idx] += 1;
            }
        }
    }

    // Calculate statistics
    let total_visits: usize = entity_visit_counts.iter().sum();
    let expected_per_entity = total_visits as f64 / num_entities as f64;
    let min_visits = *entity_visit_counts.iter().min().unwrap();
    let max_visits = *entity_visit_counts.iter().max().unwrap();

    // With shuffled selection, the ratio between max and min should be reasonable
    // (not more than 5x difference, ideally closer to 2x)
    // With sequential order, the first entity would have ~100x more visits than the last
    let coverage_ratio = if min_visits > 0 {
        max_visits as f64 / min_visits as f64
    } else {
        f64::INFINITY
    };

    eprintln!("Entity visit counts: {:?}", entity_visit_counts);
    eprintln!(
        "Total visits: {}, Expected per entity: {:.1}",
        total_visits, expected_per_entity
    );
    eprintln!(
        "Min: {}, Max: {}, Ratio: {:.2}",
        min_visits, max_visits, coverage_ratio
    );

    // Assert uniform coverage
    assert!(
        min_visits > 0,
        "All entities should be visited at least once over {} iterations",
        iterations
    );
    assert!(
        coverage_ratio < 5.0,
        "Coverage ratio should be less than 5x for uniform distribution, got {:.2}",
        coverage_ratio
    );
}
