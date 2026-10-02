/* End-to-end cross tri stream chain tests: uni → cross bi → cross tri. */

use solverforge_core::score::SoftScore;
use solverforge_core::PlanningSolution;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::joiner::equal_bi;
use crate::stream::ConstraintFactory;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct StreamShift {
    id: usize,
    employee_id: Option<usize>,
    day: u32,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct StreamEmployee {
    id: usize,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct StreamDayOff {
    employee_id: Option<usize>,
    day: u32,
}

#[derive(Clone, Debug)]
struct StreamSchedule {
    shifts: Vec<StreamShift>,
    employees: Vec<StreamEmployee>,
    days_off: Vec<StreamDayOff>,
    score: Option<SoftScore>,
}

impl PlanningSolution for StreamSchedule {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn stream_shifts(s: &StreamSchedule) -> &[StreamShift] {
    s.shifts.as_slice()
}

fn stream_employees(s: &StreamSchedule) -> &[StreamEmployee] {
    s.employees.as_slice()
}

fn stream_days_off(s: &StreamSchedule) -> &[StreamDayOff] {
    s.days_off.as_slice()
}

fn sample() -> StreamSchedule {
    StreamSchedule {
        shifts: vec![
            StreamShift {
                id: 0,
                employee_id: Some(0),
                day: 1,
            },
            StreamShift {
                id: 1,
                employee_id: Some(1),
                day: 2,
            },
            StreamShift {
                id: 2,
                employee_id: Some(0),
                day: 3,
            },
        ],
        employees: vec![StreamEmployee { id: 0 }, StreamEmployee { id: 1 }],
        days_off: vec![
            StreamDayOff {
                employee_id: Some(0),
                day: 1,
            },
            StreamDayOff {
                employee_id: Some(0),
                day: 3,
            },
        ],
        score: None,
    }
}

fn day_off_constraint() -> impl IncrementalConstraint<StreamSchedule, SoftScore> {
    use crate::stream::collection_extract::{source, ChangeSource};

    ConstraintFactory::<StreamSchedule, SoftScore>::new()
        .for_each(source(
            stream_shifts as fn(&StreamSchedule) -> &[StreamShift],
            ChangeSource::Descriptor(0),
        ))
        .join((
            source(
                stream_employees as fn(&StreamSchedule) -> &[StreamEmployee],
                ChangeSource::Descriptor(1),
            ),
            equal_bi(
                |shift: &StreamShift| shift.employee_id,
                |employee: &StreamEmployee| Some(employee.id),
            ),
        ))
        .join((
            source(
                stream_days_off as fn(&StreamSchedule) -> &[StreamDayOff],
                ChangeSource::Descriptor(2),
            ),
            |day_off: &StreamDayOff| day_off.employee_id,
        ))
        .filter(
            |shift: &StreamShift, _employee: &StreamEmployee, day_off: &StreamDayOff| {
                shift.day == day_off.day
            },
        )
        .penalize(SoftScore::of(1))
        .named("Shift on day off")
}

#[test]
fn cross_tri_chain_scores_three_source_rows() {
    let constraint = day_off_constraint();
    let schedule = sample();
    // (shift 0, emp 0, dayoff 0): day 1 == 1 → match.
    // (shift 2, emp 0, dayoff 1): day 3 == 3 → match.
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-2));
    assert_eq!(constraint.match_count(&schedule), 2);

    let matches = constraint.get_matches(&schedule);
    assert_eq!(matches.len(), 2);
}

#[test]
fn cross_tri_chain_incremental_updates_c_source() {
    let schedule = sample();
    let mut constraint = day_off_constraint();
    assert_eq!(constraint.initialize(&schedule), SoftScore::of(-2));

    // Retract the second day-off row, killing the (shift 2, emp 0, dayoff 1)
    // match; evaluate on the trimmed solution confirms the retained state.
    let trimmed = StreamSchedule {
        shifts: schedule.shifts.clone(),
        employees: schedule.employees.clone(),
        days_off: vec![StreamDayOff {
            employee_id: Some(0),
            day: 1,
        }],
        score: None,
    };
    let delta = constraint.on_retract(&trimmed, 1, 2);
    assert_eq!(delta, SoftScore::of(1));
    assert_eq!(constraint.evaluate(&trimmed), SoftScore::of(-1));
    assert_eq!(constraint.match_count(&trimmed), 1);
}
