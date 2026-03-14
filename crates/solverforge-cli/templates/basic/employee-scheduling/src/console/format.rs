// ─── Formatting helpers ───────────────────────────────────────────────────────

use std::time::Duration;
use owo_colors::OwoColorize;

pub(crate) fn format_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.2}s", d.as_secs_f64())
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{}m {}s", mins, secs)
    }
}

pub(crate) fn format_score(score: &str) -> String {
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
        }
        if n > 0 {
            return score.bright_green().to_string();
        }
    }
    score.white().to_string()
}

pub(crate) fn timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}.{:03}", d.as_secs(), d.subsec_millis()))
        .unwrap_or_else(|_| "0.000".to_string())
}

pub(crate) fn calculate_problem_scale(entity_count: usize, value_count: usize) -> String {
    if entity_count == 0 || value_count == 0 {
        return "0".to_string();
    }
    let log_scale = (entity_count as f64) * (value_count as f64).log10();
    let exponent = log_scale.floor() as i32;
    let mantissa = 10f64.powf(log_scale - exponent as f64);
    format!("{:.3} x 10^{}", mantissa, exponent)
}
