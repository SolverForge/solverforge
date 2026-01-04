//! Colorful console output for solver metrics.

use num_format::{Locale, ToFormattedString};
use owo_colors::OwoColorize;
use std::time::{Duration, Instant};

/// ASCII art banner for solver startup.
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
        "Vehicle Routing".bright_cyan()
    );
}

/// Prints "Solving started" message.
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

    // Problem scale
    let scale = calculate_problem_scale(entity_count, value_count);
    println!(
        "{} {} {} entity count ({}), variable count ({}), value count ({}), problem scale ({})",
        timestamp().bright_black(),
        "INFO".bright_green(),
        "[Solver]".bright_cyan(),
        entity_count.to_formatted_string(&Locale::en).bright_yellow(),
        variable_count.to_formatted_string(&Locale::en).bright_yellow(),
        value_count.to_formatted_string(&Locale::en).bright_yellow(),
        scale.bright_magenta()
    );
}

/// Prints a phase start message.
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

/// Prints a phase end message with metrics.
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
        "{} {} {} {} phase ({}) ended: time spent ({}), best score ({}), move evaluation speed ({}/sec), step total ({}, {:.1}% accepted)",
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

/// Prints a step progress update with moves/sec prominently displayed.
pub fn print_step_progress(
    step: u64,
    elapsed: Duration,
    moves_evaluated: u64,
    score: &str,
) {
    let moves_per_sec = if elapsed.as_secs_f64() > 0.0 {
        (moves_evaluated as f64 / elapsed.as_secs_f64()) as u64
    } else {
        0
    };

    println!(
        "    {} Step {:>7} │ {} │ {}/sec │ {}",
        "→".bright_blue(),
        step.to_formatted_string(&Locale::en).white(),
        format!("{:>6}", format_duration(elapsed)).bright_black(),
        format!("{:>8}", moves_per_sec.to_formatted_string(&Locale::en)).bright_magenta().bold(),
        format_score(score)
    );
}

/// Prints solver completion summary.
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
        "{} {} {} Solving ended: time spent ({}), best score ({}), move evaluation speed ({}/sec), phase total ({})",
        timestamp().bright_black(),
        "INFO".bright_green(),
        "[Solver]".bright_cyan(),
        format_duration(total_duration).yellow(),
        format_score(final_score),
        moves_per_sec.to_formatted_string(&Locale::en).bright_magenta().bold(),
        phase_count.to_string().white()
    );

    // Pretty summary box (60 chars wide, 56 char content area)
    println!();
    println!("{}", "╔══════════════════════════════════════════════════════════╗".bright_cyan());

    let status_text = if is_feasible {
        "✓ FEASIBLE SOLUTION FOUND"
    } else {
        "✗ INFEASIBLE (hard constraints violated)"
    };
    let status_colored = if is_feasible {
        status_text.bright_green().bold().to_string()
    } else {
        status_text.bright_red().bold().to_string()
    };
    let status_padding = 56 - status_text.chars().count();
    let left_pad = status_padding / 2;
    let right_pad = status_padding - left_pad;
    println!(
        "{}{}{}{}{}",
        "║".bright_cyan(),
        " ".repeat(left_pad),
        status_colored,
        " ".repeat(right_pad),
        "║".bright_cyan()
    );

    println!("{}", "╠══════════════════════════════════════════════════════════╣".bright_cyan());

    let score_str = final_score;
    println!(
        "{}  {:<18}{:>36}  {}",
        "║".bright_cyan(),
        "Final Score:",
        score_str,
        "║".bright_cyan()
    );

    let time_str = format!("{:.2}s", total_duration.as_secs_f64());
    println!(
        "{}  {:<18}{:>36}  {}",
        "║".bright_cyan(),
        "Solving Time:",
        time_str,
        "║".bright_cyan()
    );

    let speed_str = format!("{}/sec", moves_per_sec.to_formatted_string(&Locale::en));
    println!(
        "{}  {:<18}{:>36}  {}",
        "║".bright_cyan(),
        "Move Speed:",
        speed_str,
        "║".bright_cyan()
    );

    println!("{}", "╚══════════════════════════════════════════════════════════╝".bright_cyan());
    println!();
}

/// Prints VRP-specific configuration.
pub fn print_config(vehicles: usize, visits: usize, locations: usize) {
    println!(
        "{} {} {} Problem: vehicles ({}), visits ({}), locations ({})",
        timestamp().bright_black(),
        "INFO".bright_green(),
        "[Solver]".bright_cyan(),
        vehicles.to_formatted_string(&Locale::en).bright_yellow(),
        visits.to_formatted_string(&Locale::en).bright_yellow(),
        locations.to_formatted_string(&Locale::en).bright_yellow()
    );
}

/// Formats a duration nicely.
fn format_duration(d: Duration) -> String {
    let total_ms = d.as_millis();
    if total_ms < 1000 {
        format!("{}ms", total_ms)
    } else if total_ms < 60_000 {
        format!("{:.2}s", d.as_secs_f64())
    } else {
        let mins = total_ms / 60_000;
        let secs = (total_ms % 60_000) / 1000;
        format!("{}m {}s", mins, secs)
    }
}

/// Formats a score with colors based on feasibility.
fn format_score(score: &str) -> String {
    // Parse HardSoftScore format like "-2hard/5soft" or "0hard/10soft"
    if score.contains("hard") {
        let parts: Vec<&str> = score.split('/').collect();
        if parts.len() == 2 {
            let hard = parts[0].trim_end_matches("hard");
            let soft = parts[1].trim_end_matches("soft");

            let hard_num: f64 = hard.parse().unwrap_or(0.0);
            let soft_num: f64 = soft.parse().unwrap_or(0.0);

            let hard_str = if hard_num < 0.0 {
                format!("{}hard", hard).bright_red().to_string()
            } else {
                format!("{}hard", hard).bright_green().to_string()
            };

            let soft_str = if soft_num < 0.0 {
                format!("{}soft", soft).yellow().to_string()
            } else if soft_num > 0.0 {
                format!("{}soft", soft).bright_green().to_string()
            } else {
                format!("{}soft", soft).white().to_string()
            };

            return format!("{}/{}", hard_str, soft_str);
        }
    }

    // Simple score
    if let Ok(n) = score.parse::<i32>() {
        if n < 0 {
            return score.bright_red().to_string();
        } else if n > 0 {
            return score.bright_green().to_string();
        }
    }

    score.white().to_string()
}

/// Returns a timestamp string.
fn timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            let millis = d.subsec_millis();
            format!("{}.{:03}", secs, millis)
        })
        .unwrap_or_else(|_| "0.000".to_string())
}

/// Calculates an approximate problem scale.
fn calculate_problem_scale(entity_count: usize, value_count: usize) -> String {
    if entity_count == 0 || value_count == 0 {
        return "0".to_string();
    }

    // value_count ^ entity_count
    let log_scale = (entity_count as f64) * (value_count as f64).log10();
    let exponent = log_scale.floor() as i32;
    let mantissa = 10f64.powf(log_scale - exponent as f64);

    format!("{:.3} × 10^{}", mantissa, exponent)
}

/// A timer for tracking phase/step durations.
pub struct PhaseTimer {
    start: Instant,
    phase_name: String,
    phase_index: usize,
    steps_accepted: u64,
    moves_evaluated: u64,
    last_score: String,
}

impl PhaseTimer {
    pub fn start(phase_name: impl Into<String>, phase_index: usize) -> Self {
        let name = phase_name.into();
        print_phase_start(&name, phase_index);
        Self {
            start: Instant::now(),
            phase_name: name,
            phase_index,
            steps_accepted: 0,
            moves_evaluated: 0,
            last_score: String::new(),
        }
    }

    pub fn record_accepted(&mut self, score: &str) {
        self.steps_accepted += 1;
        self.last_score = score.to_string();
    }

    pub fn record_move(&mut self) {
        self.moves_evaluated += 1;
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn moves_evaluated(&self) -> u64 {
        self.moves_evaluated
    }

    pub fn finish(self) {
        print_phase_end(
            &self.phase_name,
            self.phase_index,
            self.start.elapsed(),
            self.steps_accepted,
            self.moves_evaluated,
            &self.last_score,
        );
    }

    pub fn steps_accepted(&self) -> u64 {
        self.steps_accepted
    }
}
