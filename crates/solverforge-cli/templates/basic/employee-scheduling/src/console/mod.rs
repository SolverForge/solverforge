mod format;
mod phase_timer;

pub use phase_timer::PhaseTimer;

use format::{calculate_problem_scale, format_duration, format_score, timestamp};
use num_format::{Locale, ToFormattedString};
use owo_colors::OwoColorize;
use std::time::Duration;

// ─── Public print functions ───────────────────────────────────────────────────

pub fn print_banner() {
    let banner = r#"
  ____        _                _____
 / ___|  ___ | |_   _____ _ __|  ___|__  _ __ __ _  ___
 \___ \ / _ \| \ \ / / _ \ '__| |_ / _ \| '__/ _` |/ _ \
  ___) | (_) | |\ V /  __/ |  |  _| (_) | | | (_| |  __/
 |____/ \___/|_| \_/ \___|_|  |_|  \___/|_|  \__, |\___|
                                              |___/
"#;
    println!("{}", banner.cyan().bold());
    println!(
        "  {} {}\n",
        format!("v{}", env!("CARGO_PKG_VERSION")).bright_black(),
        "Employee Scheduling".bright_cyan()
    );
}

pub fn print_solving_started(
    time_spent_ms: u64,
    best_score: &str,
    entity_count: usize,
    variable_count: usize,
    value_count: usize,
) {
    println!(
        "{} {} {} time spent ({}), best score ({}), random ({})",
        timestamp().bright_black(),
        "INFO".bright_green(),
        "[Solver]".bright_cyan(),
        format!("{}ms", time_spent_ms).yellow(),
        format_score(best_score),
        "StdRng".white()
    );
    let scale = calculate_problem_scale(entity_count, value_count);
    println!(
        "{} {} {} entity count ({}), variable count ({}), value count ({}), problem scale ({})",
        timestamp().bright_black(),
        "INFO".bright_green(),
        "[Solver]".bright_cyan(),
        entity_count
            .to_formatted_string(&Locale::en)
            .bright_yellow(),
        variable_count
            .to_formatted_string(&Locale::en)
            .bright_yellow(),
        value_count.to_formatted_string(&Locale::en).bright_yellow(),
        scale.bright_magenta()
    );
}

pub fn print_phase_start(phase_name: &str, phase_index: usize) {
    println!(
        "{} {} {} {} phase ({}) started",
        timestamp().bright_black(),
        "INFO".bright_green(),
        format!("[{}]", phase_name).bright_cyan(),
        phase_name.white().bold(),
        phase_index.to_string().yellow()
    );
}

pub fn print_phase_end(
    phase_name: &str,
    phase_index: usize,
    duration: Duration,
    steps_accepted: u64,
    moves_evaluated: u64,
    best_score: &str,
) {
    let moves_per_sec = if duration.as_secs_f64() > 0.0 {
        (moves_evaluated as f64 / duration.as_secs_f64()) as u64
    } else {
        0
    };
    let acceptance_rate = if moves_evaluated > 0 {
        (steps_accepted as f64 / moves_evaluated as f64) * 100.0
    } else {
        0.0
    };

    println!(
        "{} {} {} {} phase ({}) ended: time spent ({}), best score ({}), speed ({}/sec), steps ({}, {:.1}% accepted)",
        timestamp().bright_black(),
        "INFO".bright_green(),
        format!("[{}]", phase_name).bright_cyan(),
        phase_name.white().bold(),
        phase_index.to_string().yellow(),
        format_duration(duration).yellow(),
        format_score(best_score),
        moves_per_sec.to_formatted_string(&Locale::en).bright_magenta().bold(),
        steps_accepted.to_formatted_string(&Locale::en).white(),
        acceptance_rate
    );
}

pub fn print_step_progress(step: u64, elapsed: Duration, moves_evaluated: u64, score: &str) {
    let moves_per_sec = if elapsed.as_secs_f64() > 0.0 {
        (moves_evaluated as f64 / elapsed.as_secs_f64()) as u64
    } else {
        0
    };

    println!(
        "    {} Step {:>7} | {} | {}/sec | {}",
        "->".bright_blue(),
        step.to_formatted_string(&Locale::en).white(),
        format!("{:>6}", format_duration(elapsed)).bright_black(),
        format!("{:>8}", moves_per_sec.to_formatted_string(&Locale::en))
            .bright_magenta()
            .bold(),
        format_score(score)
    );
}

pub fn print_solving_ended(
    total_duration: Duration,
    total_moves: u64,
    phase_count: usize,
    final_score: &str,
    is_feasible: bool,
) {
    let moves_per_sec = if total_duration.as_secs_f64() > 0.0 {
        (total_moves as f64 / total_duration.as_secs_f64()) as u64
    } else {
        0
    };

    println!(
        "{} {} {} Solving ended: time spent ({}), best score ({}), speed ({}/sec), phase total ({})",
        timestamp().bright_black(),
        "INFO".bright_green(),
        "[Solver]".bright_cyan(),
        format_duration(total_duration).yellow(),
        format_score(final_score),
        moves_per_sec.to_formatted_string(&Locale::en).bright_magenta().bold(),
        phase_count.to_string().white()
    );

    println!();
    println!(
        "{}",
        "=============================================================".bright_cyan()
    );
    let status_text = if is_feasible {
        "  FEASIBLE SOLUTION FOUND  "
            .bright_green()
            .bold()
            .to_string()
    } else {
        "  INFEASIBLE (hard constraints violated)  "
            .bright_red()
            .bold()
            .to_string()
    };
    println!("{}", status_text);
    println!(
        "{}",
        "=============================================================".bright_cyan()
    );
    println!("  Score:   {}", final_score);
    println!("  Time:    {:.2}s", total_duration.as_secs_f64());
    println!(
        "  Speed:   {}/sec",
        moves_per_sec.to_formatted_string(&Locale::en)
    );
    println!(
        "{}",
        "=============================================================".bright_cyan()
    );
    println!();
}

pub fn print_config(shifts: usize, employees: usize) {
    println!(
        "{} {} {} shifts ({}), employees ({})",
        timestamp().bright_black(),
        "INFO".bright_green(),
        "[Solver]".bright_cyan(),
        shifts.to_formatted_string(&Locale::en).bright_yellow(),
        employees.to_formatted_string(&Locale::en).bright_yellow()
    );
}
