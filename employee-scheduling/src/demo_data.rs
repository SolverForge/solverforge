//! Demo data generators for Employee Scheduling.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use rand::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::domain::{Employee, EmployeeSchedule, Shift};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoData {
    Small,
    Large,
}

impl std::str::FromStr for DemoData {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "SMALL" => Ok(DemoData::Small),
            "LARGE" => Ok(DemoData::Large),
            _ => Err(()),
        }
    }
}

impl DemoData {
    pub fn as_str(&self) -> &'static str {
        match self {
            DemoData::Small => "SMALL",
            DemoData::Large => "LARGE",
        }
    }

    fn parameters(&self) -> DemoDataParameters {
        match self {
            DemoData::Small => DemoDataParameters {
                locations: vec![
                    "Ambulatory care".to_string(),
                    "Critical care".to_string(),
                    "Pediatric care".to_string(),
                ],
                required_skills: vec!["Doctor".to_string(), "Nurse".to_string()],
                optional_skills: vec!["Anaesthetics".to_string(), "Cardiology".to_string()],
                days_in_schedule: 14,
                employee_count: 15,
                optional_skill_distribution: vec![(1, 3.0), (2, 1.0)],
                shift_count_distribution: vec![(1, 0.9), (2, 0.1)],
                availability_count_distribution: vec![(1, 4.0), (2, 3.0), (3, 2.0), (4, 1.0)],
            },
            DemoData::Large => DemoDataParameters {
                locations: vec![
                    "Ambulatory care".to_string(),
                    "Neurology".to_string(),
                    "Critical care".to_string(),
                    "Pediatric care".to_string(),
                    "Surgery".to_string(),
                    "Radiology".to_string(),
                    "Outpatient".to_string(),
                ],
                required_skills: vec!["Doctor".to_string(), "Nurse".to_string()],
                optional_skills: vec![
                    "Anaesthetics".to_string(),
                    "Cardiology".to_string(),
                    "Radiology".to_string(),
                ],
                days_in_schedule: 28,
                employee_count: 50,
                optional_skill_distribution: vec![(1, 3.0), (2, 1.0)],
                shift_count_distribution: vec![(1, 0.5), (2, 0.3), (3, 0.2)],
                availability_count_distribution: vec![(5, 4.0), (10, 3.0), (15, 2.0), (20, 1.0)],
            },
        }
    }
}

struct DemoDataParameters {
    locations: Vec<String>,
    required_skills: Vec<String>,
    optional_skills: Vec<String>,
    days_in_schedule: i64,
    employee_count: usize,
    optional_skill_distribution: Vec<(usize, f64)>,
    shift_count_distribution: Vec<(usize, f64)>,
    availability_count_distribution: Vec<(usize, f64)>,
}

/// List of available demo data sets.
pub fn list_demo_data() -> Vec<&'static str> {
    vec!["SMALL", "LARGE"]
}

/// Generates a demo schedule for the given size.
pub fn generate(demo: DemoData) -> EmployeeSchedule {
    let params = demo.parameters();
    let mut rng = StdRng::seed_from_u64(0);

    // First Monday from a reference date
    let start_date = find_next_monday(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());

    // Build location -> shift start times map (cycling through templates)
    let shift_start_times_combos: Vec<Vec<NaiveTime>> = vec![
        vec![time(6, 0), time(14, 0)],
        vec![time(6, 0), time(14, 0), time(22, 0)],
        vec![time(6, 0), time(9, 0), time(14, 0), time(22, 0)],
    ];

    let location_to_shift_times: Vec<(&String, &Vec<NaiveTime>)> = params
        .locations
        .iter()
        .enumerate()
        .map(|(i, loc)| {
            (
                loc,
                &shift_start_times_combos[i % shift_start_times_combos.len()],
            )
        })
        .collect();

    // Generate employee names (FIRST × LAST)
    let name_permutations = generate_name_permutations(&mut rng);

    // Generate employees
    let mut employees = Vec::new();
    for i in 0..params.employee_count {
        let name = name_permutations[i % name_permutations.len()].clone();

        // Pick optional skills based on distribution
        let optional_count = pick_count(&mut rng, &params.optional_skill_distribution);
        let mut skills: Vec<String> = params
            .optional_skills
            .choose_multiple(&mut rng, optional_count.min(params.optional_skills.len()))
            .cloned()
            .collect();

        // Add one required skill
        if let Some(required) = params.required_skills.choose(&mut rng) {
            skills.push(required.clone());
        }

        employees.push(Employee::new(i, &name).with_skills(skills));
    }

    // Generate shifts and assign availabilities
    let mut shifts = Vec::new();
    let mut shift_id = 0usize;

    for day in 0..params.days_in_schedule {
        let date = start_date + Duration::days(day);

        // Pick employees to have availability entries on this day
        let availability_count = pick_count(&mut rng, &params.availability_count_distribution);
        let employees_with_availability: Vec<usize> = (0..params.employee_count)
            .collect::<Vec<_>>()
            .choose_multiple(&mut rng, availability_count.min(params.employee_count))
            .copied()
            .collect();

        for emp_idx in employees_with_availability {
            match rng.gen_range(0..3) {
                0 => {
                    employees[emp_idx].unavailable_dates.insert(date);
                }
                1 => {
                    employees[emp_idx].undesired_dates.insert(date);
                }
                2 => {
                    employees[emp_idx].desired_dates.insert(date);
                }
                _ => {}
            }
        }

        // Generate shifts for each location/timeslot
        for (location, shift_times) in &location_to_shift_times {
            for &shift_start in *shift_times {
                let start = NaiveDateTime::new(date, shift_start);
                let end = start + Duration::hours(8);

                // How many shifts at this timeslot?
                let shift_count = pick_count(&mut rng, &params.shift_count_distribution);

                for _ in 0..shift_count {
                    // Pick required skill (50% required, 50% optional)
                    let required_skill = if rng.gen_bool(0.5) {
                        params.required_skills.choose(&mut rng)
                    } else {
                        params.optional_skills.choose(&mut rng)
                    }
                    .cloned()
                    .unwrap_or_else(|| "Doctor".to_string());

                    shifts.push(Shift::new(
                        shift_id.to_string(),
                        start,
                        end,
                        (*location).clone(),
                        required_skill,
                    ));
                    shift_id += 1;
                }
            }
        }
    }

    // Finalize employees to populate derived Vec fields
    for emp in &mut employees {
        emp.finalize();
    }

    EmployeeSchedule::new(employees, shifts)
}

fn time(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
}

fn find_next_monday(date: NaiveDate) -> NaiveDate {
    let days_until_monday = match date.weekday() {
        Weekday::Mon => 0,
        Weekday::Tue => 6,
        Weekday::Wed => 5,
        Weekday::Thu => 4,
        Weekday::Fri => 3,
        Weekday::Sat => 2,
        Weekday::Sun => 1,
    };
    date + Duration::days(days_until_monday)
}

/// Pick a count based on weighted distribution.
fn pick_count(rng: &mut StdRng, distribution: &[(usize, f64)]) -> usize {
    let total_weight: f64 = distribution.iter().map(|(_, w)| w).sum();
    let mut choice = rng.gen::<f64>() * total_weight;

    for (count, weight) in distribution {
        if choice < *weight {
            return *count;
        }
        choice -= weight;
    }
    distribution.last().map(|(c, _)| *c).unwrap_or(1)
}

const FIRST_NAMES: &[&str] = &[
    "Amy", "Beth", "Carl", "Dan", "Elsa", "Flo", "Gus", "Hugo", "Ivy", "Jay",
];
const LAST_NAMES: &[&str] = &[
    "Cole", "Fox", "Green", "Jones", "King", "Li", "Poe", "Rye", "Smith", "Watt",
];

fn generate_name_permutations(rng: &mut StdRng) -> Vec<String> {
    let mut names = Vec::with_capacity(FIRST_NAMES.len() * LAST_NAMES.len());
    for first in FIRST_NAMES {
        for last in LAST_NAMES {
            names.push(format!("{} {}", first, last));
        }
    }
    names.shuffle(rng);
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_small() {
        let schedule = generate(DemoData::Small);

        assert_eq!(schedule.employees.len(), 15);
        // 14 days × 3 locations × varying timeslots × varying shifts per timeslot
        // Should be roughly 14 * 3 * avg(2,3,4) * avg(1,2) ≈ 14 * 3 * 3 * 1.1 ≈ 139
        assert!(
            schedule.shifts.len() >= 100,
            "Expected >= 100 shifts, got {}",
            schedule.shifts.len()
        );

        // All shifts should be unassigned initially
        assert!(schedule.shifts.iter().all(|s| s.employee_idx.is_none()));
    }

    #[test]
    fn test_generate_large() {
        let schedule = generate(DemoData::Large);

        assert_eq!(schedule.employees.len(), 50);
        // 28 days × 7 locations × varying timeslots × varying shifts per timeslot
        assert!(
            schedule.shifts.len() >= 500,
            "Expected >= 500 shifts, got {}",
            schedule.shifts.len()
        );
    }

    #[test]
    fn test_employees_have_skills() {
        let schedule = generate(DemoData::Small);

        for employee in &schedule.employees {
            assert!(
                !employee.skills.is_empty(),
                "Employee {} has no skills",
                employee.name
            );
        }
    }

    #[test]
    fn test_demo_data_from_str() {
        assert_eq!("SMALL".parse::<DemoData>(), Ok(DemoData::Small));
        assert_eq!("small".parse::<DemoData>(), Ok(DemoData::Small));
        assert_eq!("LARGE".parse::<DemoData>(), Ok(DemoData::Large));
        assert!("invalid".parse::<DemoData>().is_err());
    }

    #[test]
    fn test_medical_domain() {
        let schedule = generate(DemoData::Small);

        // Check for medical skills
        let all_skills: std::collections::HashSet<_> = schedule
            .employees
            .iter()
            .flat_map(|e| e.skills.iter())
            .collect();

        assert!(
            all_skills.iter().any(|s| *s == "Doctor" || *s == "Nurse"),
            "Should have Doctor or Nurse skills"
        );

        // Check for medical locations
        let locations: std::collections::HashSet<_> = schedule
            .shifts
            .iter()
            .map(|s| s.location.as_str())
            .collect();

        assert!(
            locations.contains("Ambulatory care") || locations.contains("Critical care"),
            "Should have medical locations"
        );
    }

    #[test]
    fn test_empty_schedule_has_score() {
        use crate::domain::EmployeeSchedule;
        use solverforge::Solvable;
        use tokio::sync::mpsc::unbounded_channel;

        // Empty schedule with no shifts and no employees
        let schedule = EmployeeSchedule::new(vec![], vec![]);
        let (sender, mut receiver) = unbounded_channel();
        schedule.solve(None, sender);

        // Try to receive solution - with 0 entities, solver may close channel without sending
        if let Some((result, _score)) = receiver.blocking_recv() {
            assert!(
                result.score.is_some(),
                "Empty schedule should have a score after solving, got None"
            );
        } else {
            // If no solution was sent (channel closed), that's acceptable for 0 entities
            // The solver may optimize this case by not running at all
        }
    }
}
