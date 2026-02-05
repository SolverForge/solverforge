//! Test utilities for solverforge-scoring
//!
//! Provides common test fixtures used across the crate's test modules.

use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use std::any::TypeId;

/// A queen entity for N-Queens problem tests.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Queen {
    pub id: i64,
    pub row: i64,
    pub col: i64,
}

impl Queen {
    /// Creates a new queen at the given position.
    pub fn new(id: i64, row: i64, col: i64) -> Self {
        Self { id, row, col }
    }

    /// Creates a queen with only row and col (id defaults to row).
    pub fn at(row: i64, col: i64) -> Self {
        Self { id: row, row, col }
    }
}

/// N-Queens solution for testing.
#[derive(Clone, Debug)]
pub struct NQueensSolution {
    pub queens: Vec<Queen>,
    pub score: Option<SimpleScore>,
}

impl NQueensSolution {
    /// Creates an empty N-Queens solution.
    pub fn empty() -> Self {
        Self {
            queens: Vec::new(),
            score: None,
        }
    }

    /// Creates a solution with the given queens.
    pub fn with_queens(queens: Vec<Queen>) -> Self {
        Self {
            queens,
            score: None,
        }
    }

    /// Returns a reference to the queens.
    pub fn queens(&self) -> &Vec<Queen> {
        &self.queens
    }

    /// Returns a mutable reference to the queens.
    pub fn queens_mut(&mut self) -> &mut Vec<Queen> {
        &mut self.queens
    }

    /// Calculates the number of queen conflicts (row, column, diagonal).
    pub fn calculate_conflicts(&self) -> i64 {
        let mut conflicts = 0i64;
        for i in 0..self.queens.len() {
            for j in (i + 1)..self.queens.len() {
                let q1 = &self.queens[i];
                let q2 = &self.queens[j];

                // Same row
                if q1.row == q2.row {
                    conflicts += 1;
                }
                // Same column
                if q1.col == q2.col {
                    conflicts += 1;
                }
                // Same diagonal
                if (q1.row - q2.row).abs() == (q1.col - q2.col).abs() {
                    conflicts += 1;
                }
            }
        }
        conflicts
    }
}

impl PlanningSolution for NQueensSolution {
    type Score = SimpleScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

/// Gets a reference to the queens vector.
pub fn get_queens(s: &NQueensSolution) -> &Vec<Queen> {
    &s.queens
}

/// Gets a mutable reference to the queens vector.
pub fn get_queens_mut(s: &mut NQueensSolution) -> &mut Vec<Queen> {
    &mut s.queens
}

/// A shift entity for scheduling tests.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Shift {
    pub id: usize,
    pub employee_id: Option<usize>,
    pub start: i64,
    pub end: i64,
}

impl Shift {
    /// Creates a new shift.
    pub fn new(id: usize, employee_id: Option<usize>, start: i64, end: i64) -> Self {
        Self {
            id,
            employee_id,
            start,
            end,
        }
    }

    /// Creates an unassigned shift.
    pub fn unassigned(id: usize, start: i64, end: i64) -> Self {
        Self {
            id,
            employee_id: None,
            start,
            end,
        }
    }

    /// Creates an assigned shift.
    pub fn assigned(id: usize, employee_id: usize, start: i64, end: i64) -> Self {
        Self {
            id,
            employee_id: Some(employee_id),
            start,
            end,
        }
    }
}

/// An employee entity for scheduling tests.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Employee {
    pub id: usize,
    pub name: String,
}

impl Employee {
    /// Creates a new employee.
    pub fn new(id: usize, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }
}

/// A schedule solution for testing.
#[derive(Clone, Debug)]
pub struct Schedule {
    pub shifts: Vec<Shift>,
    pub employees: Vec<Employee>,
    pub score: Option<SimpleScore>,
}

impl Schedule {
    /// Creates an empty schedule.
    pub fn empty() -> Self {
        Self {
            shifts: Vec::new(),
            employees: Vec::new(),
            score: None,
        }
    }

    /// Creates a schedule with shifts and employees.
    pub fn new(shifts: Vec<Shift>, employees: Vec<Employee>) -> Self {
        Self {
            shifts,
            employees,
            score: None,
        }
    }

    /// Returns a reference to the shifts.
    pub fn shifts(&self) -> &Vec<Shift> {
        &self.shifts
    }

    /// Returns a mutable reference to the shifts.
    pub fn shifts_mut(&mut self) -> &mut Vec<Shift> {
        &mut self.shifts
    }
}

impl PlanningSolution for Schedule {
    type Score = SimpleScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

/// Gets a reference to the shifts vector.
pub fn get_shifts(s: &Schedule) -> &Vec<Shift> {
    &s.shifts
}

/// Gets a mutable reference to the shifts vector.
pub fn get_shifts_mut(s: &mut Schedule) -> &mut Vec<Shift> {
    &mut s.shifts
}

/// Creates a SolutionDescriptor for NQueensSolution.
pub fn create_nqueens_descriptor() -> SolutionDescriptor {
    let extractor = Box::new(TypedEntityExtractor::new(
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

/// Creates a SolutionDescriptor for Schedule.
pub fn create_schedule_descriptor() -> SolutionDescriptor {
    let extractor = Box::new(TypedEntityExtractor::new(
        "Shift",
        "shifts",
        get_shifts,
        get_shifts_mut,
    ));

    let entity_desc =
        EntityDescriptor::new("Shift", TypeId::of::<Shift>(), "shifts").with_extractor(extractor);

    SolutionDescriptor::new("Schedule", TypeId::of::<Schedule>()).with_entity(entity_desc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queen_creation() {
        let q1 = Queen::new(1, 0, 0);
        assert_eq!(q1.id, 1);
        assert_eq!(q1.row, 0);
        assert_eq!(q1.col, 0);

        let q2 = Queen::at(2, 3);
        assert_eq!(q2.id, 2);
        assert_eq!(q2.row, 2);
        assert_eq!(q2.col, 3);
    }

    #[test]
    fn test_nqueens_no_conflicts() {
        // Valid 4-queens solution
        let solution = NQueensSolution::with_queens(vec![
            Queen::at(0, 1),
            Queen::at(1, 3),
            Queen::at(2, 0),
            Queen::at(3, 2),
        ]);
        assert_eq!(solution.calculate_conflicts(), 0);
    }

    #[test]
    fn test_nqueens_with_conflicts() {
        // Two queens in same row
        let solution = NQueensSolution::with_queens(vec![Queen::at(0, 0), Queen::at(0, 1)]);
        assert_eq!(solution.calculate_conflicts(), 1);
    }

    #[test]
    fn test_shift_creation() {
        let s1 = Shift::unassigned(1, 0, 8);
        assert!(s1.employee_id.is_none());

        let s2 = Shift::assigned(2, 5, 8, 16);
        assert_eq!(s2.employee_id, Some(5));
    }

    #[test]
    fn test_schedule_creation() {
        let schedule = Schedule::new(
            vec![Shift::unassigned(1, 0, 8)],
            vec![Employee::new(1, "Alice")],
        );
        assert_eq!(schedule.shifts.len(), 1);
        assert_eq!(schedule.employees.len(), 1);
    }

    #[test]
    fn test_nqueens_descriptor() {
        let desc = create_nqueens_descriptor();
        assert_eq!(desc.entity_descriptor_count(), 1);
    }
}
