use super::support::*;

use solverforge_core::domain::PlanningSolution;

use crate::stream::joiner::equal_bi;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Lesson {
    index: usize,
    group_idx: usize,
    timeslot_idx: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Timeslot {
    index: usize,
    start: i64,
    end: i64,
}

#[derive(Clone)]
struct Timetable {
    lessons: Vec<Lesson>,
    timeslots: Vec<Timeslot>,
}

impl PlanningSolution for Timetable {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        None
    }

    fn set_score(&mut self, _score: Option<Self::Score>) {}
}

#[derive(Debug, PartialEq, Eq)]
struct AssignedLessonSlot {
    lesson_idx: usize,
    group_idx: usize,
    start: i64,
    end: i64,
}

fn lessons(timetable: &Timetable) -> &[Lesson] {
    timetable.lessons.as_slice()
}

fn timeslots(timetable: &Timetable) -> &[Timeslot] {
    timetable.timeslots.as_slice()
}

fn overlapping_lessons_constraint(
    timeslot_source: ChangeSource,
) -> impl IncrementalConstraint<Timetable, SoftScore> {
    ConstraintFactory::<Timetable, SoftScore>::new()
        .for_each(source(
            lessons as fn(&Timetable) -> &[Lesson],
            ChangeSource::Descriptor(0),
        ))
        .join((
            source(timeslots as fn(&Timetable) -> &[Timeslot], timeslot_source),
            equal_bi(
                |lesson: &Lesson| lesson.timeslot_idx,
                |timeslot: &Timeslot| Some(timeslot.index),
            ),
        ))
        .project(|lesson: &Lesson, timeslot: &Timeslot| AssignedLessonSlot {
            lesson_idx: lesson.index,
            group_idx: lesson.group_idx,
            start: timeslot.start,
            end: timeslot.end,
        })
        .join(equal(|row: &AssignedLessonSlot| row.group_idx))
        .filter(|left: &AssignedLessonSlot, right: &AssignedLessonSlot| {
            left.lesson_idx != right.lesson_idx && left.start < right.end && right.start < left.end
        })
        .penalize(|_left: &AssignedLessonSlot, _right: &AssignedLessonSlot| SoftScore::of(1))
        .named("overlapping group lessons")
}

fn assigned_lesson_penalty(
    lesson_source: ChangeSource,
    timeslot_source: ChangeSource,
) -> impl IncrementalConstraint<Timetable, SoftScore> {
    ConstraintFactory::<Timetable, SoftScore>::new()
        .for_each(source(
            lessons as fn(&Timetable) -> &[Lesson],
            lesson_source,
        ))
        .join((
            source(timeslots as fn(&Timetable) -> &[Timeslot], timeslot_source),
            equal_bi(
                |lesson: &Lesson| lesson.timeslot_idx,
                |timeslot: &Timeslot| Some(timeslot.index),
            ),
        ))
        .project(|lesson: &Lesson, timeslot: &Timeslot| AssignedLessonSlot {
            lesson_idx: lesson.index,
            group_idx: lesson.group_idx,
            start: timeslot.start,
            end: timeslot.end,
        })
        .penalize(|_row: &AssignedLessonSlot| SoftScore::of(1))
        .named("assigned lesson slots")
}

fn overlapping_timetable() -> Timetable {
    Timetable {
        lessons: vec![
            Lesson {
                index: 0,
                group_idx: 0,
                timeslot_idx: Some(0),
            },
            Lesson {
                index: 1,
                group_idx: 0,
                timeslot_idx: Some(1),
            },
        ],
        timeslots: vec![
            Timeslot {
                index: 0,
                start: 0,
                end: 10,
            },
            Timeslot {
                index: 1,
                start: 5,
                end: 15,
            },
            Timeslot {
                index: 2,
                start: 10,
                end: 20,
            },
        ],
    }
}

#[test]
fn joined_projected_rows_score_timetabling_overlap() {
    let constraint = overlapping_lessons_constraint(ChangeSource::Descriptor(1));
    let timetable = overlapping_timetable();

    assert_eq!(constraint.match_count(&timetable), 1);
    assert_eq!(constraint.evaluate(&timetable), SoftScore::of(-1));
}

#[test]
fn joined_projected_rows_score_director_updates_after_lesson_timeslot_change() {
    let mut director = ScoreDirector::new(
        overlapping_timetable(),
        (overlapping_lessons_constraint(ChangeSource::Descriptor(1)),),
    );
    assert_eq!(director.calculate_score(), SoftScore::of(-1));

    director.before_variable_changed(0, 1);
    director.working_solution_mut().lessons[1].timeslot_idx = Some(2);
    director.after_variable_changed(0, 1);

    assert_eq!(director.calculate_score(), SoftScore::ZERO);
    assert_eq!(
        director.calculate_score(),
        director
            .constraints()
            .evaluate_all(director.working_solution())
    );
}

#[test]
fn joined_projected_rows_emit_nothing_for_unassigned_or_missing_keys() {
    let constraint = overlapping_lessons_constraint(ChangeSource::Descriptor(1));
    let timetable = Timetable {
        lessons: vec![
            Lesson {
                index: 0,
                group_idx: 0,
                timeslot_idx: None,
            },
            Lesson {
                index: 1,
                group_idx: 0,
                timeslot_idx: Some(99),
            },
        ],
        timeslots: vec![Timeslot {
            index: 0,
            start: 0,
            end: 10,
        }],
    };

    assert_eq!(constraint.match_count(&timetable), 0);
    assert_eq!(constraint.evaluate(&timetable), SoftScore::ZERO);
}

#[test]
fn joined_projected_rows_right_descriptor_updates_refresh_score() {
    let mut director = ScoreDirector::new(
        overlapping_timetable(),
        (overlapping_lessons_constraint(ChangeSource::Descriptor(1)),),
    );
    assert_eq!(director.calculate_score(), SoftScore::of(-1));

    director.before_variable_changed(1, 1);
    {
        let timeslot = &mut director.working_solution_mut().timeslots[1];
        timeslot.start = 10;
        timeslot.end = 20;
    }
    director.after_variable_changed(1, 1);

    assert_eq!(director.calculate_score(), SoftScore::ZERO);
    assert_eq!(
        director.calculate_score(),
        director
            .constraints()
            .evaluate_all(director.working_solution())
    );
}

#[test]
fn joined_projected_rows_static_right_source_ignores_right_callbacks() {
    let mut constraint = overlapping_lessons_constraint(ChangeSource::Static);
    let timetable = overlapping_timetable();

    let mut total = constraint.initialize(&timetable);
    assert_eq!(total, SoftScore::of(-1));

    total = total + constraint.on_retract(&timetable, 1, 1);
    total = total + constraint.on_insert(&timetable, 1, 1);

    assert_eq!(total, SoftScore::of(-1));
}

#[test]
fn joined_projected_rows_same_descriptor_owners_do_not_double_delta() {
    let mut constraint =
        assigned_lesson_penalty(ChangeSource::Descriptor(0), ChangeSource::Descriptor(0));
    let timetable = Timetable {
        lessons: vec![Lesson {
            index: 0,
            group_idx: 0,
            timeslot_idx: Some(0),
        }],
        timeslots: vec![Timeslot {
            index: 0,
            start: 0,
            end: 10,
        }],
    };

    let mut total = constraint.initialize(&timetable);
    assert_eq!(total, SoftScore::of(-1));

    total = total + constraint.on_retract(&timetable, 0, 0);
    assert_eq!(total, SoftScore::ZERO);

    total = total + constraint.on_insert(&timetable, 0, 0);
    assert_eq!(total, SoftScore::of(-1));
}

#[test]
#[should_panic(expected = "cannot localize entity indexes")]
fn joined_projected_rows_unknown_source_panics_on_localized_callback() {
    let mut constraint =
        assigned_lesson_penalty(ChangeSource::Unknown, ChangeSource::Descriptor(1));
    let timetable = overlapping_timetable();

    constraint.initialize(&timetable);
    constraint.on_retract(&timetable, 0, 0);
}
