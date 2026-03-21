/* Built-in demo data generator.

   Generates sample employees and shifts so the app runs out of the box
   without requiring external CSV files. Matches the quickstart's DemoData
   generator (same seed, same algorithm, same output). */

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use rand::rngs::StdRng;
use rand::RngExt;
use rand::seq::{IndexedRandom, SliceRandom};
use rand::SeedableRng;

use crate::domain::{Employee, EmployeeSchedule, Shift};

/// Selects Small (15 employees, 14 days) or Large (50 employees, 28 days) demo.
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

/// Generates a demo `EmployeeSchedule` for the given size.
pub fn generate(demo: DemoData) -> EmployeeSchedule {
    let params = parameters(demo);
    let mut rng = StdRng::seed_from_u64(0);
    let start_date = next_monday(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());

    let shift_time_combos: Vec<Vec<NaiveTime>> = vec![
        vec![time(6, 0), time(14, 0)],
        vec![time(6, 0), time(14, 0), time(22, 0)],
        vec![time(6, 0), time(9, 0), time(14, 0), time(22, 0)],
    ];

    let location_times: Vec<(&String, &Vec<NaiveTime>)> = params
        .locations
        .iter()
        .enumerate()
        .map(|(i, loc)| (loc, &shift_time_combos[i % shift_time_combos.len()]))
        .collect();

    let name_pool = name_permutations(&mut rng);

    // --- Generate employees ---
    let mut employees = Vec::with_capacity(params.employee_count);
    for i in 0..params.employee_count {
        let name = name_pool[i % name_pool.len()].clone();
        let opt_count = pick_count(&mut rng, &params.optional_skill_dist);
        let mut skills: Vec<String> = params
            .optional_skills
            .sample(&mut rng, opt_count.min(params.optional_skills.len()))
            .cloned()
            .collect();
        if let Some(req) = params.required_skills.choose(&mut rng) {
            skills.push(req.clone());
        }
        employees.push(Employee::new(i, &name).with_skills(skills));
    }

    // --- Assign availability dates ---
    for day in 0..params.days {
        let date = start_date + Duration::days(day);
        let avail_count = pick_count(&mut rng, &params.availability_dist);
        let chosen: Vec<usize> = (0..params.employee_count)
            .collect::<Vec<_>>()
            .sample(&mut rng, avail_count.min(params.employee_count))
            .copied()
            .collect();

        for emp_idx in chosen {
            match rng.random_range(0..3) {
                0 => {
                    employees[emp_idx].unavailable_dates.insert(date);
                }
                1 => {
                    employees[emp_idx].undesired_dates.insert(date);
                }
                _ => {
                    employees[emp_idx].desired_dates.insert(date);
                }
            }
        }
    }

    // --- Generate shifts ---
    let mut shifts = Vec::new();
    let mut shift_id = 0usize;

    for day in 0..params.days {
        let date = start_date + Duration::days(day);
        for (location, times) in &location_times {
            for &t in *times {
                let start = NaiveDateTime::new(date, t);
                let end = start + Duration::hours(8);
                let count = pick_count(&mut rng, &params.shift_count_dist);
                for _ in 0..count {
                    let skill = if rng.random_bool(0.5) {
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
                        skill,
                    ));
                    shift_id += 1;
                }
            }
        }
    }

    // Finalize employees: populate sorted Vec fields from HashSets
    for emp in &mut employees {
        emp.finalize();
    }

    EmployeeSchedule::new(employees, shifts)
}

// ============================================================================
// Parameters
// ============================================================================

struct Params {
    locations: Vec<String>,
    required_skills: Vec<String>,
    optional_skills: Vec<String>,
    days: i64,
    employee_count: usize,
    optional_skill_dist: Vec<(usize, f64)>,
    shift_count_dist: Vec<(usize, f64)>,
    availability_dist: Vec<(usize, f64)>,
}

fn parameters(demo: DemoData) -> Params {
    match demo {
        DemoData::Small => Params {
            locations: vec![
                "Ambulatory care".into(),
                "Critical care".into(),
                "Pediatric care".into(),
            ],
            required_skills: vec!["Doctor".into(), "Nurse".into()],
            optional_skills: vec!["Anaesthetics".into(), "Cardiology".into()],
            days: 14,
            employee_count: 15,
            optional_skill_dist: vec![(1, 3.0), (2, 1.0)],
            shift_count_dist: vec![(1, 0.9), (2, 0.1)],
            availability_dist: vec![(1, 4.0), (2, 3.0), (3, 2.0), (4, 1.0)],
        },
        DemoData::Large => Params {
            locations: vec![
                "Ambulatory care".into(),
                "Neurology".into(),
                "Critical care".into(),
                "Pediatric care".into(),
                "Surgery".into(),
                "Radiology".into(),
                "Outpatient".into(),
            ],
            required_skills: vec!["Doctor".into(), "Nurse".into()],
            optional_skills: vec![
                "Anaesthetics".into(),
                "Cardiology".into(),
                "Radiology".into(),
            ],
            days: 28,
            employee_count: 50,
            optional_skill_dist: vec![(1, 3.0), (2, 1.0)],
            shift_count_dist: vec![(1, 0.5), (2, 0.3), (3, 0.2)],
            availability_dist: vec![(5, 4.0), (10, 3.0), (15, 2.0), (20, 1.0)],
        },
    }
}

// ============================================================================
// Utilities
// ============================================================================

fn time(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

fn next_monday(date: NaiveDate) -> NaiveDate {
    let days = match date.weekday() {
        Weekday::Mon => 0,
        Weekday::Tue => 6,
        Weekday::Wed => 5,
        Weekday::Thu => 4,
        Weekday::Fri => 3,
        Weekday::Sat => 2,
        Weekday::Sun => 1,
    };
    date + Duration::days(days)
}

fn pick_count(rng: &mut StdRng, dist: &[(usize, f64)]) -> usize {
    let total: f64 = dist.iter().map(|(_, w)| w).sum();
    let mut choice = rng.random::<f64>() * total;
    for (count, weight) in dist {
        if choice < *weight {
            return *count;
        }
        choice -= weight;
    }
    dist.last().map(|(c, _)| *c).unwrap_or(1)
}

const FIRST_NAMES: &[&str] = &[
    "Amy", "Beth", "Carl", "Dan", "Elsa", "Flo", "Gus", "Hugo", "Ivy", "Jay",
];
const LAST_NAMES: &[&str] = &[
    "Cole", "Fox", "Green", "Jones", "King", "Li", "Poe", "Rye", "Smith", "Watt",
];

fn name_permutations(rng: &mut StdRng) -> Vec<String> {
    let mut names = Vec::with_capacity(FIRST_NAMES.len() * LAST_NAMES.len());
    for first in FIRST_NAMES {
        for last in LAST_NAMES {
            names.push(format!("{} {}", first, last));
        }
    }
    names.shuffle(rng);
    names
}
