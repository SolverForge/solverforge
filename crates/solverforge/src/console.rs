//! Colorful console output for solver metrics.
//!
//! Provides a custom `tracing` layer that formats solver events with colors.
//! Auto-initialized when the `console` feature is enabled.

use num_format::{Locale, ToFormattedString};
use owo_colors::OwoColorize;
use std::io::{self, Write};
use std::sync::OnceLock;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

static INIT: OnceLock<()> = OnceLock::new();

/// Initializes the solver console output.
///
/// Safe to call multiple times - only the first call has effect.
/// Prints the SolverForge banner and sets up tracing.
pub fn init() {
    INIT.get_or_init(|| {
        print_banner();

        let filter = EnvFilter::from_default_env()
            .add_directive("solverforge_solver=info".parse().unwrap());

        tracing_subscriber::registry()
            .with(filter)
            .with(SolverConsoleLayer)
            .init();
    });
}

fn print_banner() {
    use std::io::Write;

    let banner = r#"
 ____        _                 _____
/ ___|  ___ | |_   _____ _ __ |  ___|__  _ __ __ _  ___
\___ \ / _ \| \ \ / / _ \ '__|| |_ / _ \| '__/ _` |/ _ \
 ___) | (_) | |\ V /  __/ |   |  _| (_) | | | (_| |  __/
|____/ \___/|_| \_/ \___|_|   |_|  \___/|_|  \__, |\___|
                                             |___/
"#;

    let version_line = format!(
        "                   v{} - Zero-Erasure Constraint Solver\n",
        env!("CARGO_PKG_VERSION")
    );

    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{}", banner.bright_cyan());
    let _ = writeln!(stdout, "{}", version_line.bright_white().bold());
    let _ = stdout.flush();
}

/// A tracing layer that formats solver events with colors.
pub struct SolverConsoleLayer;

impl<S: Subscriber> Layer<S> for SolverConsoleLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let target = metadata.target();

        // Only handle solverforge_solver events
        if !target.starts_with("solverforge_solver") {
            return;
        }

        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        let output = format_solver_event(&visitor);
        if !output.is_empty() {
            let _ = writeln!(io::stdout(), "{}", output);
        }
    }
}

#[derive(Default)]
struct EventVisitor {
    message: Option<String>,
    phase_name: Option<String>,
    phase_index: Option<u64>,
    duration_ms: Option<u64>,
    steps: Option<u64>,
    moves_evaluated: Option<u64>,
    moves_per_sec: Option<u64>,
    best_score: Option<String>,
    final_score: Option<String>,
    entity_count: Option<u64>,
    variable_count: Option<u64>,
    value_count: Option<u64>,
    step: Option<u64>,
    score: Option<String>,
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let s = format!("{:?}", value);
        match field.name() {
            "message" => self.message = Some(s),
            "phase_name" => self.phase_name = Some(s.trim_matches('"').to_string()),
            "best_score" => self.best_score = Some(s.trim_matches('"').to_string()),
            "final_score" => self.final_score = Some(s.trim_matches('"').to_string()),
            "score" => self.score = Some(s.trim_matches('"').to_string()),
            _ => {}
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        match field.name() {
            "phase_index" => self.phase_index = Some(value),
            "duration_ms" => self.duration_ms = Some(value),
            "steps" => self.steps = Some(value),
            "moves_evaluated" => self.moves_evaluated = Some(value),
            "moves_per_sec" => self.moves_per_sec = Some(value),
            "entity_count" => self.entity_count = Some(value),
            "variable_count" => self.variable_count = Some(value),
            "value_count" => self.value_count = Some(value),
            "step" => self.step = Some(value),
            _ => {}
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record_u64(field, value as u64);
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "message" => self.message = Some(value.to_string()),
            "phase_name" => self.phase_name = Some(value.to_string()),
            "best_score" => self.best_score = Some(value.to_string()),
            "final_score" => self.final_score = Some(value.to_string()),
            "score" => self.score = Some(value.to_string()),
            _ => {}
        }
    }
}

fn format_solver_event(v: &EventVisitor) -> String {
    let msg = v.message.as_deref().unwrap_or("");

    match msg {
        "Solving started" => format_solving_started(v),
        "Phase started" => format_phase_start(v),
        "Phase ended" => format_phase_end(v),
        "Solving ended" => format_solving_ended(v),
        "New best solution" => format_new_best(v),
        _ => String::new(),
    }
}

fn format_solving_started(v: &EventVisitor) -> String {
    let entity_count = v.entity_count.unwrap_or(0);
    let variable_count = v.variable_count.unwrap_or(entity_count);
    let value_count = v.value_count.unwrap_or(0);

    let scale = calculate_problem_scale(entity_count as usize, value_count as usize);

    format!(
        "{} {} {} entity count ({}), variable count ({}), value count ({}), problem scale ({})",
        timestamp().bright_black(),
        "INFO".bright_green(),
        "[Solver]".bright_cyan(),
        entity_count.to_formatted_string(&Locale::en).bright_yellow(),
        variable_count.to_formatted_string(&Locale::en).bright_yellow(),
        value_count.to_formatted_string(&Locale::en).bright_yellow(),
        scale.bright_magenta()
    )
}

fn format_phase_start(v: &EventVisitor) -> String {
    let phase_name = v.phase_name.as_deref().unwrap_or("Unknown");
    let phase_index = v.phase_index.unwrap_or(0);

    format!(
        "{} {} {} {} phase ({}) started",
        timestamp().bright_black(),
        "INFO".bright_green(),
        format!("[{}]", phase_name).bright_cyan(),
        phase_name.white().bold(),
        phase_index.to_string().yellow()
    )
}

fn format_phase_end(v: &EventVisitor) -> String {
    let phase_name = v.phase_name.as_deref().unwrap_or("Unknown");
    let phase_index = v.phase_index.unwrap_or(0);
    let duration_ms = v.duration_ms.unwrap_or(0);
    let steps = v.steps.unwrap_or(0);
    let moves_per_sec = v.moves_per_sec.unwrap_or(0);
    let best_score = v.best_score.as_deref().unwrap_or("N/A");

    format!(
        "{} {} {} {} phase ({}) ended: time spent ({}), best score ({}), move evaluation speed ({}/sec), step total ({})",
        timestamp().bright_black(),
        "INFO".bright_green(),
        format!("[{}]", phase_name).bright_cyan(),
        phase_name.white().bold(),
        phase_index.to_string().yellow(),
        format_duration_ms(duration_ms).yellow(),
        format_score(best_score),
        moves_per_sec.to_formatted_string(&Locale::en).bright_magenta().bold(),
        steps.to_formatted_string(&Locale::en).white()
    )
}

fn format_solving_ended(v: &EventVisitor) -> String {
    let final_score = v.final_score.as_deref().unwrap_or("N/A");
    let is_feasible = !final_score.contains('-') || final_score.starts_with("0hard");

    let mut output = format!(
        "{} {} {} Solving ended: best score ({})",
        timestamp().bright_black(),
        "INFO".bright_green(),
        "[Solver]".bright_cyan(),
        format_score(final_score)
    );

    // Pretty summary box
    output.push_str("\n\n");
    output.push_str(&"╔══════════════════════════════════════════════════════════╗".bright_cyan().to_string());
    output.push('\n');

    let status_text = if is_feasible {
        "FEASIBLE SOLUTION FOUND"
    } else {
        "INFEASIBLE (hard constraints violated)"
    };
    let status_colored = if is_feasible {
        format!("  {}  ", status_text).bright_green().bold().to_string()
    } else {
        format!("  {}  ", status_text).bright_red().bold().to_string()
    };
    let status_padding = 56 - status_text.len() - 4;
    let left_pad = status_padding / 2;
    let right_pad = status_padding - left_pad;
    output.push_str(&format!(
        "{}{}{}{}{}",
        "║".bright_cyan(),
        " ".repeat(left_pad),
        status_colored,
        " ".repeat(right_pad),
        "║".bright_cyan()
    ));
    output.push('\n');

    output.push_str(&"╠══════════════════════════════════════════════════════════╣".bright_cyan().to_string());
    output.push('\n');

    output.push_str(&format!(
        "{}  {:<18}{:>36}  {}",
        "║".bright_cyan(),
        "Final Score:",
        final_score,
        "║".bright_cyan()
    ));
    output.push('\n');

    output.push_str(&"╚══════════════════════════════════════════════════════════╝".bright_cyan().to_string());
    output.push('\n');

    output
}

fn format_new_best(v: &EventVisitor) -> String {
    let step = v.step.unwrap_or(0);
    let score = v.score.as_deref().unwrap_or("N/A");

    format!(
        "    {} Step {:>7} | {}",
        "->".bright_blue(),
        step.to_formatted_string(&Locale::en).white(),
        format_score(score)
    )
}

fn timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs() % 100000;
            let millis = d.subsec_millis();
            format!("{:5}.{:03}", secs, millis)
        })
        .unwrap_or_else(|_| "    0.000".to_string())
}

fn format_duration_ms(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.2}s", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{}m {}s", mins, secs)
    }
}

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

fn calculate_problem_scale(entity_count: usize, value_count: usize) -> String {
    if entity_count == 0 || value_count == 0 {
        return "0".to_string();
    }

    // value_count ^ entity_count
    let log_scale = (entity_count as f64) * (value_count as f64).log10();
    let exponent = log_scale.floor() as i32;
    let mantissa = 10f64.powf(log_scale - exponent as f64);

    format!("{:.3} x 10^{}", mantissa, exponent)
}
