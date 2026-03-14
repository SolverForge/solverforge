/* Lightweight CSV <-> domain struct conversion.

   Reads and writes simple CSV files without any heavy dependencies.

   employees.csv columns:
     name, skills, unavailable_dates, undesired_dates, desired_dates
     (list columns use comma-separated values within quoted cells)

   shifts.csv columns:
     id, start, end, location, required_skill, employee_name */

use crate::domain::{Employee, EmployeeSchedule, Shift};
use chrono::{NaiveDate, NaiveDateTime};
use std::collections::HashSet;
use std::io::{BufRead, Write};

/// Loads employees and shifts from CSV files and builds an `EmployeeSchedule`.
pub fn load_schedule(employees_path: &str, shifts_path: &str) -> Result<EmployeeSchedule, String> {
    let employees = load_employees_csv(employees_path)?;
    let shifts = load_shifts_csv(shifts_path, &employees)?;
    Ok(EmployeeSchedule::new(employees, shifts))
}

/// Writes a solved `EmployeeSchedule` to CSV files.
pub fn save_schedule(
    schedule: &EmployeeSchedule,
    employees_path: &str,
    shifts_path: &str,
) -> Result<(), String> {
    save_employees_csv(&schedule.employees, employees_path)?;
    save_shifts_csv(&schedule.shifts, &schedule.employees, shifts_path)?;
    Ok(())
}

// ============================================================================
// CSV -> domain structs
// ============================================================================

/// Loads employees from a CSV file.
pub fn load_employees_csv(path: &str) -> Result<Vec<Employee>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("failed to open {}: {}", path, e))?;
    let reader = std::io::BufReader::new(file);
    let mut employees = Vec::new();

    for (line_no, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| format!("{}:{}: {}", path, line_no + 1, e))?;
        if line_no == 0 {
            continue; // skip header
        }
        let fields = parse_csv_line(&line);
        if fields.len() < 2 {
            continue;
        }

        let name = fields[0].clone();
        let skills: HashSet<String> = parse_list(&fields.get(1).cloned().unwrap_or_default());
        let unavailable_dates: HashSet<NaiveDate> =
            parse_date_list(&fields.get(2).cloned().unwrap_or_default());
        let undesired_dates: HashSet<NaiveDate> =
            parse_date_list(&fields.get(3).cloned().unwrap_or_default());
        let desired_dates: HashSet<NaiveDate> =
            parse_date_list(&fields.get(4).cloned().unwrap_or_default());

        let index = employees.len();
        let mut emp = Employee {
            index,
            name,
            skills,
            unavailable_dates,
            undesired_dates,
            desired_dates,
            unavailable_days: Vec::new(),
            undesired_days: Vec::new(),
            desired_days: Vec::new(),
        };
        emp.finalize();
        employees.push(emp);
    }

    Ok(employees)
}

/// Loads shifts from a CSV file, resolving employee names to indices.
pub fn load_shifts_csv(path: &str, employees: &[Employee]) -> Result<Vec<Shift>, String> {
    let name_to_idx: std::collections::HashMap<&str, usize> = employees
        .iter()
        .map(|e| (e.name.as_str(), e.index))
        .collect();

    let file = std::fs::File::open(path).map_err(|e| format!("failed to open {}: {}", path, e))?;
    let reader = std::io::BufReader::new(file);
    let mut shifts = Vec::new();

    for (line_no, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| format!("{}:{}: {}", path, line_no + 1, e))?;
        if line_no == 0 {
            continue; // skip header
        }
        let fields = parse_csv_line(&line);
        if fields.len() < 5 {
            continue;
        }

        let id = fields[0].clone();
        let start = parse_datetime(&fields[1]).ok_or_else(|| {
            format!(
                "{}:{}: invalid start datetime '{}'",
                path,
                line_no + 1,
                fields[1]
            )
        })?;
        let end = parse_datetime(&fields[2]).ok_or_else(|| {
            format!(
                "{}:{}: invalid end datetime '{}'",
                path,
                line_no + 1,
                fields[2]
            )
        })?;
        let location = fields[3].clone();
        let required_skill = fields[4].clone();
        let employee_idx = fields
            .get(5)
            .filter(|s| !s.is_empty())
            .and_then(|name| name_to_idx.get(name.as_str()).copied());

        shifts.push(Shift {
            id,
            start,
            end,
            location,
            required_skill,
            employee_idx,
        });
    }

    Ok(shifts)
}

// ============================================================================
// Domain structs -> CSV
// ============================================================================

fn save_employees_csv(employees: &[Employee], path: &str) -> Result<(), String> {
    let mut file =
        std::fs::File::create(path).map_err(|e| format!("failed to create {}: {}", path, e))?;
    writeln!(
        file,
        "name,skills,unavailable_dates,undesired_dates,desired_dates"
    )
    .map_err(|e| e.to_string())?;

    for emp in employees {
        let skills = format_set_sorted(&emp.skills);
        let unavail = format_dates(&emp.unavailable_days);
        let undesired = format_dates(&emp.undesired_days);
        let desired = format_dates(&emp.desired_days);
        writeln!(
            file,
            "{},\"{}\",\"{}\",\"{}\",\"{}\"",
            emp.name, skills, unavail, undesired, desired
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn save_shifts_csv(shifts: &[Shift], employees: &[Employee], path: &str) -> Result<(), String> {
    let mut file =
        std::fs::File::create(path).map_err(|e| format!("failed to create {}: {}", path, e))?;
    writeln!(file, "id,start,end,location,required_skill,employee_name")
        .map_err(|e| e.to_string())?;

    for shift in shifts {
        let emp_name = shift
            .employee_idx
            .and_then(|idx| employees.get(idx))
            .map(|e| e.name.as_str())
            .unwrap_or("");
        writeln!(
            file,
            "{},{},{},{},{},{}",
            shift.id,
            shift.start.format("%Y-%m-%d %H:%M:%S"),
            shift.end.format("%Y-%m-%d %H:%M:%S"),
            shift.location,
            shift.required_skill,
            emp_name
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ============================================================================
// CSV parsing helpers
// ============================================================================

/// Parses a CSV line respecting quoted fields (handles commas inside quotes).
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current.trim().to_string());
    fields
}

/// Parses a comma-separated list of values from a single cell.
fn parse_list(s: &str) -> HashSet<String> {
    s.split(',')
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

/// Parses a comma-separated list of dates from a single cell.
fn parse_date_list(s: &str) -> HashSet<NaiveDate> {
    s.split(',')
        .filter_map(|v| v.trim().parse::<NaiveDate>().ok())
        .collect()
}

/// Parses a datetime string in "YYYY-MM-DD HH:MM:SS" or "YYYY-MM-DDTHH:MM:SS" format.
fn parse_datetime(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim();
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .ok()
}

fn format_dates(dates: &[NaiveDate]) -> String {
    dates
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn format_set_sorted(set: &HashSet<String>) -> String {
    let mut v: Vec<&str> = set.iter().map(|s| s.as_str()).collect();
    v.sort();
    v.join(",")
}
