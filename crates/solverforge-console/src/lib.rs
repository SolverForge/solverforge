//! Colorful console output for solver metrics.
//!
//! Provides a custom `tracing` layer that formats solver events with colors.
//!
//! ## Log Levels
//!
//! - **INFO**: Lifecycle events (solving/phase start/end)
//! - **DEBUG**: Progress updates (1/sec with speed and score)
//! - **TRACE**: Individual step evaluations

use num_format::{Locale, ToFormattedString};
use owo_colors::OwoColorize;
use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

static INIT: OnceLock<()> = OnceLock::new();
static EPOCH: OnceLock<Instant> = OnceLock::new();
static SOLVE_START_NANOS: AtomicU64 = AtomicU64::new(0);

/// Package version for banner display.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initializes the solver console output.
///
/// Safe to call multiple times - only the first call has effect.
/// Prints the SolverForge banner and sets up tracing.
pub fn init() {
    INIT.get_or_init(|| {
        print_banner();

        let filter = EnvFilter::builder()
            .with_default_directive("solverforge_solver=info".parse().unwrap())
            .from_env_lossy()
            .add_directive("solverforge_dynamic=info".parse().unwrap());

        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(SolverConsoleLayer)
            .try_init();
    });
}

// Marks the start of solving for elapsed time tracking.
fn mark_solve_start() {
    let epoch = EPOCH.get_or_init(Instant::now);
    let nanos = epoch.elapsed().as_nanos() as u64;
    SOLVE_START_NANOS.store(nanos, Ordering::Relaxed);
}

// Returns elapsed time since solve start.
fn elapsed_secs() -> f64 {
    let Some(epoch) = EPOCH.get() else {
        return 0.0;
    };
    let start_nanos = SOLVE_START_NANOS.load(Ordering::Relaxed);
    let now_nanos = epoch.elapsed().as_nanos() as u64;
    (now_nanos - start_nanos) as f64 / 1_000_000_000.0
}

fn print_banner() {
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
        VERSION
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

        // Accept events from solver modules
        if !target.starts_with("solverforge_solver")
            && !target.starts_with("solverforge_dynamic")
            && !target.starts_with("solverforge_py")
            && !target.starts_with("solverforge::")
        {
            return;
        }

        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        let level = *metadata.level();
        let output = format_event(&visitor, level);
        if !output.is_empty() {
            let _ = writeln!(io::stdout(), "{}", output);
        }
    }
}

#[derive(Default)]
struct EventVisitor {
    event: Option<String>,
    phase: Option<String>,
    phase_index: Option<u64>,
    steps: Option<u64>,
    speed: Option<u64>,
    score: Option<String>,
    step: Option<u64>,
    entity: Option<u64>,
    accepted: Option<bool>,
    duration_ms: Option<u64>,
    entity_count: Option<u64>,
    value_count: Option<u64>,
    constraint_count: Option<u64>,
    time_limit_secs: Option<u64>,
    feasible: Option<bool>,
    moves_speed: Option<u64>,
    calc_speed: Option<u64>,
    acceptance_rate: Option<String>,
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let s = format!("{:?}", value);
        match field.name() {
            "event" => self.event = Some(s.trim_matches('"').to_string()),
            "phase" => self.phase = Some(s.trim_matches('"').to_string()),
            "score" => self.score = Some(s.trim_matches('"').to_string()),
            _ => {}
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        match field.name() {
            "phase_index" => self.phase_index = Some(value),
            "steps" => self.steps = Some(value),
            "speed" => self.speed = Some(value),
            "step" => self.step = Some(value),
            "entity" => self.entity = Some(value),
            "duration_ms" => self.duration_ms = Some(value),
            "entity_count" => self.entity_count = Some(value),
            "value_count" => self.value_count = Some(value),
            "constraint_count" => self.constraint_count = Some(value),
            "time_limit_secs" => self.time_limit_secs = Some(value),
            "moves_speed" => self.moves_speed = Some(value),
            "calc_speed" => self.calc_speed = Some(value),
            _ => {}
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record_u64(field, value as u64);
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        match field.name() {
            "accepted" => self.accepted = Some(value),
            "feasible" => self.feasible = Some(value),
            _ => {}
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "event" => self.event = Some(value.to_string()),
            "phase" => self.phase = Some(value.to_string()),
            "score" => self.score = Some(value.to_string()),
            "acceptance_rate" => self.acceptance_rate = Some(value.to_string()),
            _ => {}
        }
    }
}

fn format_event(v: &EventVisitor, level: Level) -> String {
    let event = v.event.as_deref().unwrap_or("");

    match event {
        "solve_start" => format_solve_start(v),
        "solve_end" => format_solve_end(v),
        "phase_start" => format_phase_start(v),
        "phase_end" => format_phase_end(v),
        "progress" => format_progress(v),
        "step" => format_step(v, level),
        _ => String::new(),
    }
}

fn format_elapsed() -> String {
    format!("{:>7.3}s", elapsed_secs())
        .bright_black()
        .to_string()
}

fn format_solve_start(v: &EventVisitor) -> String {
    mark_solve_start();
    let entities = v.entity_count.unwrap_or(0);
    let values = v.value_count.unwrap_or(0);
    let constraints = v.constraint_count.unwrap_or(0);
    let time_limit = v.time_limit_secs.unwrap_or(0);
    let scale = calculate_problem_scale(entities as usize, values as usize);

    let mut output = format!(
        "{} {} Solving │ {} entities │ {} values │ scale {}",
        format_elapsed(),
        "▶".bright_green().bold(),
        entities.to_formatted_string(&Locale::en).bright_yellow(),
        values.to_formatted_string(&Locale::en).bright_yellow(),
        scale.bright_magenta()
    );

    if constraints > 0 {
        output.push_str(&format!(
            " │ {} constraints",
            constraints.to_formatted_string(&Locale::en).bright_yellow()
        ));
    }

    if time_limit > 0 {
        output.push_str(&format!(
            " │ {}s limit",
            time_limit.to_formatted_string(&Locale::en).bright_yellow()
        ));
    }

    output
}

fn format_solve_end(v: &EventVisitor) -> String {
    let score = v.score.as_deref().unwrap_or("N/A");
    let is_feasible = v
        .feasible
        .unwrap_or_else(|| !score.contains('-') || score.starts_with("0hard"));

    let status = if is_feasible {
        "FEASIBLE".bright_green().bold().to_string()
    } else {
        "INFEASIBLE".bright_red().bold().to_string()
    };

    let mut output = format!(
        "{} {} Solving complete │ {} │ {}",
        format_elapsed(),
        "■".bright_cyan().bold(),
        format_score(score),
        status
    );

    // Summary box
    output.push_str("\n\n");
    output.push_str(
        &"╔══════════════════════════════════════════════════════════╗"
            .bright_cyan()
            .to_string(),
    );
    output.push('\n');

    let status_text = if is_feasible {
        "FEASIBLE SOLUTION FOUND"
    } else {
        "INFEASIBLE (hard constraints violated)"
    };
    let inner_width: usize = 58;
    let total_pad = inner_width.saturating_sub(status_text.len());
    let left_pad = total_pad / 2;
    let right_pad = total_pad - left_pad;
    let status_colored = if is_feasible {
        status_text.bright_green().bold().to_string()
    } else {
        status_text.bright_red().bold().to_string()
    };
    output.push_str(&format!(
        "{}{}{}{}{}",
        "║".bright_cyan(),
        " ".repeat(left_pad),
        status_colored,
        " ".repeat(right_pad),
        "║".bright_cyan()
    ));
    output.push('\n');

    output.push_str(
        &"╠══════════════════════════════════════════════════════════╣"
            .bright_cyan()
            .to_string(),
    );
    output.push('\n');

    output.push_str(&format!(
        "{}  {:<18}{:>36}  {}",
        "║".bright_cyan(),
        "Final Score:",
        score,
        "║".bright_cyan()
    ));
    output.push('\n');

    output.push_str(
        &"╚══════════════════════════════════════════════════════════╝"
            .bright_cyan()
            .to_string(),
    );
    output.push('\n');

    output
}

fn format_phase_start(v: &EventVisitor) -> String {
    let phase = v.phase.as_deref().unwrap_or("Unknown");

    format!(
        "{} {} {} started",
        format_elapsed(),
        "▶".bright_blue(),
        phase.white().bold()
    )
}

fn format_phase_end(v: &EventVisitor) -> String {
    let phase = v.phase.as_deref().unwrap_or("Unknown");
    let steps = v.steps.unwrap_or(0);
    let moves_speed = v.moves_speed.unwrap_or(v.speed.unwrap_or(0));
    let score = v.score.as_deref().unwrap_or("N/A");
    let duration = v.duration_ms.unwrap_or(0);

    let mut output = format!(
        "{} {} {} ended │ {} │ {} steps │ {} moves/s",
        format_elapsed(),
        "◀".bright_blue(),
        phase.white().bold(),
        format_duration_ms(duration).yellow(),
        steps.to_formatted_string(&Locale::en).white(),
        moves_speed
            .to_formatted_string(&Locale::en)
            .bright_magenta()
            .bold(),
    );

    if let Some(calc_speed) = v.calc_speed {
        output.push_str(&format!(
            " │ {} calcs/s",
            calc_speed
                .to_formatted_string(&Locale::en)
                .bright_magenta()
                .bold()
        ));
    }

    if let Some(ref rate) = v.acceptance_rate {
        output.push_str(&format!(" │ {} accepted", rate.bright_yellow()));
    }

    output.push_str(&format!(" │ {}", format_score(score)));

    output
}

fn format_progress(v: &EventVisitor) -> String {
    let steps = v.steps.unwrap_or(0);
    let speed = v.speed.unwrap_or(0);
    let score = v.score.as_deref().unwrap_or("N/A");

    format!(
        "{} {} {:>10} steps │ {:>12}/s │ {}",
        format_elapsed(),
        "⚡".bright_cyan(),
        steps.to_formatted_string(&Locale::en).white(),
        speed
            .to_formatted_string(&Locale::en)
            .bright_magenta()
            .bold(),
        format_score(score)
    )
}

fn format_step(v: &EventVisitor, level: Level) -> String {
    if level != Level::TRACE {
        return String::new();
    }

    let step = v.step.unwrap_or(0);
    let entity = v.entity.unwrap_or(0);
    let score = v.score.as_deref().unwrap_or("N/A");
    let accepted = v.accepted.unwrap_or(false);

    let icon = if accepted {
        "✓".bright_green().to_string()
    } else {
        "✗".bright_red().to_string()
    };

    format!(
        "{} {} Step {:>10} │ Entity {:>6} │ {}",
        format_elapsed(),
        icon,
        step.to_formatted_string(&Locale::en).bright_black(),
        entity.to_formatted_string(&Locale::en).bright_black(),
        format_score(score).bright_black()
    )
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

    let log_scale = (entity_count as f64) * (value_count as f64).log10();
    let exponent = log_scale.floor() as i32;
    let mantissa = 10f64.powf(log_scale - exponent as f64);

    format!("{:.3} x 10^{}", mantissa, exponent)
}
