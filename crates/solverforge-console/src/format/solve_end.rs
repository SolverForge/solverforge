use super::*;

pub(super) fn format_solve_end(v: &EventVisitor) -> String {
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
