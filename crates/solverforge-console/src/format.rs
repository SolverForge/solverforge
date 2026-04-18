// Formatting functions for solver console output.

use std::time::Duration;

use crate::time::{elapsed, mark_solve_start};
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
    format!("{:>9}", format_elapsed_duration(elapsed()))
        .bright_black()
        .to_string()
}

fn format_elapsed_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let nanos = duration.subsec_nanos();

    if secs >= 60 {
        let mins = secs / 60;
        let rem_secs = secs % 60;
        return format!("{mins}m {rem_secs}s");
    }

    if secs > 0 {
        let millis = nanos / 1_000_000;
        if millis == 0 {
            return format!("{secs}s");
        }
        return format!("{secs}s {millis}ms");
    }

    let millis = nanos / 1_000_000;
    if millis > 0 {
        return format!("{millis}ms");
    }

    let micros = nanos / 1_000;
    if micros > 0 {
        return format!("{micros}us");
    }

    format!("{nanos}ns")
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
    let moves_generated = v.moves_generated.unwrap_or(0);
    let moves_evaluated = v.moves_evaluated.unwrap_or(0);
    let moves_accepted = v.moves_accepted.unwrap_or(0);
    let score_calculations = v.score_calculations.unwrap_or(0);
    let acceptance_rate = v.acceptance_rate.as_deref().unwrap_or("0.0%");
    let generation_time = v.generation_time.as_deref();
    let evaluation_time = v.evaluation_time.as_deref();
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
        "Moves Generated:",
        moves_generated.to_formatted_string(&Locale::en),
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
    if let Some(generation_time) = generation_time {
        output.push_str(&format!(
            "{}  {:<18}{:>36}  {}",
            "║".bright_cyan(),
            "Generation Time:",
            generation_time,
            "║".bright_cyan()
        ));
        output.push('\n');
    }
    if let Some(evaluation_time) = evaluation_time {
        output.push_str(&format!(
            "{}  {:<18}{:>36}  {}",
            "║".bright_cyan(),
            "Evaluation Time:",
            evaluation_time,
            "║".bright_cyan()
        ));
        output.push('\n');
    }
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
    let moves_generated = v.moves_generated.unwrap_or(0);
    let moves_evaluated = v.moves_evaluated.unwrap_or(0);
    let moves_accepted = v.moves_accepted.unwrap_or(0);
    let score_calculations = v.score_calculations.unwrap_or(0);
    let score = v.score.as_deref().unwrap_or("N/A");
    let duration = v.duration.as_deref().unwrap_or("0ns");
    let generation_time = v.generation_time.as_deref();
    let evaluation_time = v.evaluation_time.as_deref();

    let mut output = format!(
        "{} {} {} ended │ {} │ {} steps │ {} moves/s",
        format_elapsed(),
        "◀".bright_blue(),
        phase.white().bold(),
        duration.yellow(),
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

    if moves_generated > 0 {
        output.push_str(&format!(
            " │ {} generated",
            moves_generated
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

    if let Some(generation_time) = generation_time {
        output.push_str(&format!(" │ gen {}", generation_time.bright_black()));
    }

    if let Some(evaluation_time) = evaluation_time {
        output.push_str(&format!(" │ eval {}", evaluation_time.bright_black()));
    }

    output.push_str(&format!(" │ {}", format_score(score)));

    output
}

fn format_progress(v: &EventVisitor) -> String {
    let steps = v.steps.unwrap_or(0);
    let speed = v.speed.unwrap_or(0);
    let moves_generated = v.moves_generated.unwrap_or(0);
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

    if moves_generated > 0 {
        output.push_str(&format!(
            " │ {} generated",
            moves_generated.to_formatted_string(&Locale::en).white()
        ));
    }

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
#[path = "format_tests.rs"]
mod tests;
