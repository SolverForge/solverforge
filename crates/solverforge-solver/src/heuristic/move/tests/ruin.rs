//! Tests for RuinMove operations.

use super::*;

#[derive(Clone, Debug)]
struct Task {
    assigned_to: Option<i32>,
}

#[derive(Clone, Debug)]
struct Schedule {
    tasks: Vec<Task>,
    score: Option<SimpleScore>,
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

fn get_tasks(s: &Schedule) -> &Vec<Task> {
    &s.tasks
}
fn get_tasks_mut(s: &mut Schedule) -> &mut Vec<Task> {
    &mut s.tasks
}

fn get_assigned(s: &Schedule, idx: usize) -> Option<i32> {
    s.tasks.get(idx).and_then(|t| t.assigned_to)
}
fn set_assigned(s: &mut Schedule, idx: usize, v: Option<i32>) {
    if let Some(t) = s.tasks.get_mut(idx) {
        t.assigned_to = v;
    }
}

fn create_director(
    assignments: &[Option<i32>],
) -> SimpleScoreDirector<Schedule, impl Fn(&Schedule) -> SimpleScore> {
    let tasks: Vec<Task> = assignments
        .iter()
        .map(|&a| Task { assigned_to: a })
        .collect();
    let solution = Schedule { tasks, score: None };
    let extractor = Box::new(TypedEntityExtractor::new(
        "Task",
        "tasks",
        get_tasks,
        get_tasks_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);
    let descriptor =
        SolutionDescriptor::new("Schedule", TypeId::of::<Schedule>()).with_entity(entity_desc);
    SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
}

#[test]
fn ruin_single_entity() {
    let mut director = create_director(&[Some(1), Some(2), Some(3)]);

    let m = RuinMove::<Schedule, i32>::new(&[1], get_assigned, set_assigned, "assigned_to", 0);

    assert!(m.is_doable(&director));

    {
        let mut recording = RecordingScoreDirector::new(&mut director);
        m.do_move(&mut recording);

        assert_eq!(get_assigned(recording.working_solution(), 0), Some(1));
        assert_eq!(get_assigned(recording.working_solution(), 1), None);
        assert_eq!(get_assigned(recording.working_solution(), 2), Some(3));

        recording.undo_changes();
    }

    assert_eq!(get_assigned(director.working_solution(), 1), Some(2));
}

#[test]
fn ruin_multiple_entities() {
    let mut director = create_director(&[Some(1), Some(2), Some(3), Some(4)]);

    let m =
        RuinMove::<Schedule, i32>::new(&[0, 2, 3], get_assigned, set_assigned, "assigned_to", 0);

    assert!(m.is_doable(&director));
    assert_eq!(m.ruin_count(), 3);

    {
        let mut recording = RecordingScoreDirector::new(&mut director);
        m.do_move(&mut recording);

        assert_eq!(get_assigned(recording.working_solution(), 0), None);
        assert_eq!(get_assigned(recording.working_solution(), 1), Some(2));
        assert_eq!(get_assigned(recording.working_solution(), 2), None);
        assert_eq!(get_assigned(recording.working_solution(), 3), None);

        recording.undo_changes();
    }

    assert_eq!(get_assigned(director.working_solution(), 0), Some(1));
    assert_eq!(get_assigned(director.working_solution(), 2), Some(3));
    assert_eq!(get_assigned(director.working_solution(), 3), Some(4));
}

#[test]
fn ruin_already_unassigned_is_doable() {
    let director = create_director(&[Some(1), None]);

    let m = RuinMove::<Schedule, i32>::new(&[0, 1], get_assigned, set_assigned, "assigned_to", 0);

    assert!(m.is_doable(&director));
}

#[test]
fn ruin_all_unassigned_not_doable() {
    let director = create_director(&[None, None]);

    let m = RuinMove::<Schedule, i32>::new(&[0, 1], get_assigned, set_assigned, "assigned_to", 0);

    assert!(!m.is_doable(&director));
}
