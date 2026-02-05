//! Tests for PillarSwapMove operations.

use super::*;

#[derive(Clone, Debug)]
struct Employee {
    id: usize,
    shift: Option<i32>,
}

#[derive(Clone, Debug)]
struct Solution {
    employees: Vec<Employee>,
    score: Option<SimpleScore>,
}

impl PlanningSolution for Solution {
    type Score = SimpleScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_shift(s: &Solution, idx: usize) -> Option<i32> {
    s.employees.get(idx).and_then(|e| e.shift)
}

fn set_shift(s: &mut Solution, idx: usize, v: Option<i32>) {
    if let Some(e) = s.employees.get_mut(idx) {
        e.shift = v;
    }
}

fn create_director(
    employees: Vec<Employee>,
) -> SimpleScoreDirector<Solution, impl Fn(&Solution) -> SimpleScore> {
    let solution = Solution {
        employees,
        score: None,
    };
    let extractor = Box::new(TypedEntityExtractor::new(
        "Employee",
        "employees",
        |s: &Solution| &s.employees,
        |s: &mut Solution| &mut s.employees,
    ));
    let entity_desc = EntityDescriptor::new("Employee", TypeId::of::<Employee>(), "employees")
        .with_extractor(extractor);
    let descriptor =
        SolutionDescriptor::new("Solution", TypeId::of::<Solution>()).with_entity(entity_desc);
    SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
}

#[test]
fn test_pillar_swap_all_entities() {
    let mut director = create_director(vec![
        Employee {
            id: 0,
            shift: Some(1),
        },
        Employee {
            id: 1,
            shift: Some(1),
        },
        Employee {
            id: 2,
            shift: Some(2),
        },
        Employee {
            id: 3,
            shift: Some(2),
        },
    ]);

    let m = PillarSwapMove::<Solution, i32>::new(
        vec![0, 1],
        vec![2, 3],
        get_shift,
        set_shift,
        "shift",
        0,
    );
    assert!(m.is_doable(&director));

    {
        let mut recording = RecordingScoreDirector::new(&mut director);
        m.do_move(&mut recording);

        assert_eq!(get_shift(recording.working_solution(), 0), Some(2));
        assert_eq!(get_shift(recording.working_solution(), 1), Some(2));
        assert_eq!(get_shift(recording.working_solution(), 2), Some(1));
        assert_eq!(get_shift(recording.working_solution(), 3), Some(1));

        recording.undo_changes();
    }

    assert_eq!(get_shift(director.working_solution(), 0), Some(1));
    assert_eq!(get_shift(director.working_solution(), 1), Some(1));
    assert_eq!(get_shift(director.working_solution(), 2), Some(2));
    assert_eq!(get_shift(director.working_solution(), 3), Some(2));

    let solution = director.working_solution();
    assert_eq!(solution.employees[0].id, 0);
    assert_eq!(solution.employees[1].id, 1);
    assert_eq!(solution.employees[2].id, 2);
    assert_eq!(solution.employees[3].id, 3);
}

#[test]
fn test_pillar_swap_same_value_not_doable() {
    let director = create_director(vec![
        Employee {
            id: 0,
            shift: Some(1),
        },
        Employee {
            id: 1,
            shift: Some(1),
        },
    ]);
    let m =
        PillarSwapMove::<Solution, i32>::new(vec![0], vec![1], get_shift, set_shift, "shift", 0);
    assert!(!m.is_doable(&director));
}

#[test]
fn test_pillar_swap_empty_pillar_not_doable() {
    let director = create_director(vec![Employee {
        id: 0,
        shift: Some(1),
    }]);
    let m = PillarSwapMove::<Solution, i32>::new(vec![], vec![0], get_shift, set_shift, "shift", 0);
    assert!(!m.is_doable(&director));
}
