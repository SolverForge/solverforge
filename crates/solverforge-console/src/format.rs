// Formatting functions for solver console output.

use crate::time::{elapsed_secs, mark_solve_start};
use crate::visitor::EventVisitor;
use num_format::{Locale, ToFormattedString};
use owo_colors::OwoColorize;
use tracing::Level;

pub(crate) fn format_event(v: &EventVisitor, level: Level) -> String {
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
    let value_label = if v.solve_shape.as_deref() == Some("list") {
        "elements"
    } else {
        "values"
    };
    let constraints = v.constraint_count.unwrap_or(0);
    let time_limit = v.time_limit_secs.unwrap_or(0);
    let scale = calculate_problem_scale(entities as usize, values as usize);

    let mut output = format!(
        "{} {} Solving │ {} entities │ {} {} │ scale {}",
        format_elapsed(),
        "▶".bright_green().bold(),
        entities.to_formatted_string(&Locale::en).bright_yellow(),
        values.to_formatted_string(&Locale::en).bright_yellow(),
        value_label,
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
    let steps = v.steps.unwrap_or(0);
    let moves_speed = v.moves_speed.or(v.speed).unwrap_or(0);
    let moves_evaluated = v.moves_evaluated.unwrap_or(0);
    let moves_accepted = v.moves_accepted.unwrap_or(0);
    let score_calculations = v.score_calculations.unwrap_or(0);
    let acceptance_rate = v.acceptance_rate.as_deref().unwrap_or("0.0%");
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
    output.push_str(&format!(
        "{}  {:<18}{:>36}  {}",
        "║".bright_cyan(),
        "Steps:",
        steps.to_formatted_string(&Locale::en),
        "║".bright_cyan()
    ));
    output.push('\n');
    output.push_str(&format!(
        "{}  {:<18}{:>36}  {}",
        "║".bright_cyan(),
        "Moves/s:",
        moves_speed.to_formatted_string(&Locale::en),
        "║".bright_cyan()
    ));
    output.push('\n');
    output.push_str(&format!(
        "{}  {:<18}{:>36}  {}",
        "║".bright_cyan(),
        "Moves Evaluated:",
        moves_evaluated.to_formatted_string(&Locale::en),
        "║".bright_cyan()
    ));
    output.push('\n');
    output.push_str(&format!(
        "{}  {:<18}{:>36}  {}",
        "║".bright_cyan(),
        "Moves Accepted:",
        moves_accepted.to_formatted_string(&Locale::en),
        "║".bright_cyan()
    ));
    output.push('\n');
    output.push_str(&format!(
        "{}  {:<18}{:>36}  {}",
        "║".bright_cyan(),
        "Score Calcs:",
        score_calculations.to_formatted_string(&Locale::en),
        "║".bright_cyan()
    ));
    output.push('\n');
    output.push_str(&format!(
        "{}  {:<18}{:>36}  {}",
        "║".bright_cyan(),
        "Acceptance:",
        acceptance_rate,
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
    let moves_evaluated = v.moves_evaluated.unwrap_or(0);
    let moves_accepted = v.moves_accepted.unwrap_or(0);
    let score_calculations = v.score_calculations.unwrap_or(0);
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

    if moves_evaluated > 0 {
        output.push_str(&format!(
            " │ {} moves",
            moves_evaluated
                .to_formatted_string(&Locale::en)
                .bright_white()
        ));
    }

    if moves_accepted > 0 {
        output.push_str(&format!(
            " │ {} accepted moves",
            moves_accepted
                .to_formatted_string(&Locale::en)
                .bright_white()
        ));
    }

    if score_calculations > 0 {
        output.push_str(&format!(
            " │ {} calcs",
            score_calculations
                .to_formatted_string(&Locale::en)
                .bright_white()
        ));
    }

    output.push_str(&format!(" │ {}", format_score(score)));

    output
}

fn format_progress(v: &EventVisitor) -> String {
    let steps = v.steps.unwrap_or(0);
    let speed = v.speed.unwrap_or(0);
    let moves_evaluated = v.moves_evaluated.unwrap_or(0);
    let moves_accepted = v.moves_accepted.unwrap_or(0);
    let score_calculations = v.score_calculations.unwrap_or(0);
    let current_score = v.current_score.as_deref().unwrap_or("N/A");
    let best_score = v.best_score.as_deref();
    let acceptance_rate = v.acceptance_rate.as_deref().unwrap_or("0.0%");

    let mut output = format!(
        "{} {} {:>10} steps │ {:>12}/s │ {} moves │ {} accepted │ {} calcs │ {} │ {}",
        format_elapsed(),
        "⚡".bright_cyan(),
        steps.to_formatted_string(&Locale::en).white(),
        speed
            .to_formatted_string(&Locale::en)
            .bright_magenta()
            .bold(),
        moves_evaluated.to_formatted_string(&Locale::en).white(),
        moves_accepted.to_formatted_string(&Locale::en).white(),
        score_calculations.to_formatted_string(&Locale::en).white(),
        acceptance_rate.bright_yellow(),
        format_score(current_score)
    );

    if let Some(best) = best_score.filter(|best| *best != current_score) {
        output.push_str(&format!(" │ best {}", format_score(best)));
    }

    output
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing::{Event, Level, Subscriber};
    use tracing_subscriber::layer::{Context, SubscriberExt};
    use tracing_subscriber::{Layer, Registry};

    #[derive(Clone)]
    struct CaptureLayer {
        outputs: Arc<Mutex<Vec<String>>>,
    }

    impl<S: Subscriber> Layer<S> for CaptureLayer {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut visitor = EventVisitor::default();
            event.record(&mut visitor);

            let output = format_event(&visitor, *event.metadata().level());
            if !output.is_empty() {
                self.outputs.lock().unwrap().push(output);
            }
        }
    }

    fn capture_events(f: impl FnOnce()) -> Vec<String> {
        let outputs = Arc::new(Mutex::new(Vec::new()));
        let subscriber = Registry::default().with(CaptureLayer {
            outputs: outputs.clone(),
        });

        tracing::subscriber::with_default(subscriber, f);
        let captured = outputs.lock().unwrap().clone();
        captured
    }

    #[test]
    fn format_duration_covers_milliseconds_seconds_and_minutes() {
        assert_eq!(format_duration_ms(750), "750ms");
        assert_eq!(format_duration_ms(2_500), "2.50s");
        assert_eq!(format_duration_ms(125_000), "2m 5s");
    }

    #[test]
    fn calculate_problem_scale_handles_zero_and_nonzero_inputs() {
        assert_eq!(calculate_problem_scale(0, 10), "0");
        assert_eq!(calculate_problem_scale(10, 100), "1.000 x 10^20");
    }

    #[test]
    fn format_score_handles_hard_soft_and_simple_scores() {
        let hard_soft = format_score("-2hard/5soft");
        assert!(hard_soft.contains("-2hard"));
        assert!(hard_soft.contains("5soft"));

        let simple = format_score("-7");
        assert!(simple.contains("-7"));

        let fallback = format_score("N/A");
        assert!(fallback.contains("N/A"));
    }

    #[test]
    fn format_event_renders_progress_and_trace_steps() {
        let progress = EventVisitor {
            event: Some("progress".to_string()),
            steps: Some(12_345),
            speed: Some(678),
            current_score: Some("0hard/9soft".to_string()),
            ..EventVisitor::default()
        };
        let progress_output = format_event(&progress, Level::INFO);
        assert!(progress_output.contains("steps"));
        assert!(progress_output.contains("678"));
        assert!(progress_output.contains("0hard"));

        let outputs = capture_events(|| {
            tracing::trace!(
                target: "solverforge_solver::test",
                event = "step",
                step = 42u64,
                move_index = 3u64,
                score = "-1hard/0soft",
                accepted = true,
            );
        });

        let step_output = outputs
            .iter()
            .find(|output| output.contains("Step"))
            .cloned()
            .expect("expected trace step output");
        assert!(step_output.contains("Step"));
        assert!(step_output.contains("Entity"));
        assert!(step_output.contains("3"));
        assert!(step_output.contains("-1hard"));
    }

    #[test]
    fn format_event_renders_solve_start_and_end_summaries() {
        let outputs = capture_events(|| {
            tracing::info!(
                target: "solverforge_solver::test",
                event = "solve_start",
                entity_count = 120u64,
                solve_shape = "list",
                value_count = 25u64,
                constraint_count = 7u64,
                time_limit_secs = 30u64,
            );
        });

        let start_output = outputs
            .iter()
            .find(|output| output.contains("Solving"))
            .cloned()
            .expect("expected solve_start output");
        assert!(start_output.contains("Solving"));
        assert!(start_output.contains("elements"));
        assert!(start_output.contains("120"));
        assert!(start_output.contains("25"));
        assert!(start_output.contains("constraints"));

        let end = EventVisitor {
            event: Some("solve_end".to_string()),
            score: Some("0hard/-15soft".to_string()),
            ..EventVisitor::default()
        };
        let end_output = format_event(&end, Level::INFO);
        assert!(end_output.contains("Solving complete"));
        assert!(end_output.contains("FEASIBLE"));
        assert!(end_output.contains("Final Score:"));
    }
}
