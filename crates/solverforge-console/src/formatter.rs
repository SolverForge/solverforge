//! Formatting utilities for console output.

use std::time::Duration;

/// Formats a duration in a human-readable format.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use solverforge_console::formatter::format_duration;
///
/// assert_eq!(format_duration(Duration::from_secs(0)), "0.00s");
/// assert_eq!(format_duration(Duration::from_secs(1)), "1.00s");
/// assert_eq!(format_duration(Duration::from_millis(1500)), "1.50s");
/// assert_eq!(format_duration(Duration::from_secs(65)), "1m 5s");
/// assert_eq!(format_duration(Duration::from_secs(3665)), "1h 1m 5s");
/// ```
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let millis = duration.subsec_millis();

    if total_secs >= 3600 {
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        let secs = total_secs % 60;
        format!("{}h {}m {}s", hours, mins, secs)
    } else if total_secs >= 60 {
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}.{:02}s", total_secs, millis / 10)
    }
}

/// Formats a number with thousands separators.
///
/// Inserts commas every 3 digits from right to left.
///
/// # Examples
///
/// ```
/// use solverforge_console::formatter::format_number;
///
/// assert_eq!(format_number(0), "0");
/// assert_eq!(format_number(1000), "1,000");
/// assert_eq!(format_number(1234567), "1,234,567");
/// ```
pub fn format_number(n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }

    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, ch) in chars.iter().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(*ch);
    }

    result.chars().rev().collect()
}

/// Formats a rate (items per second).
///
/// # Examples
///
/// ```
/// use solverforge_console::formatter::format_rate;
///
/// assert_eq!(format_rate(1234), "1,234/sec");
/// assert_eq!(format_rate(0), "0/sec");
/// ```
pub fn format_rate(rate: u64) -> String {
    format!("{}/sec", format_number(rate))
}

/// Calculates rate from count and duration.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use solverforge_console::formatter::calculate_rate;
///
/// assert_eq!(calculate_rate(1000, Duration::from_secs(1)), 1000);
/// assert_eq!(calculate_rate(2000, Duration::from_secs(2)), 1000);
/// assert_eq!(calculate_rate(500, Duration::from_millis(500)), 1000);
/// assert_eq!(calculate_rate(0, Duration::from_secs(1)), 0);
/// ```
pub fn calculate_rate(count: u64, elapsed: Duration) -> u64 {
    let secs = elapsed.as_secs_f64();
    if secs > 0.0 {
        (count as f64 / secs) as u64
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0.00s");
        assert_eq!(format_duration(Duration::from_millis(123)), "0.12s");
        assert_eq!(format_duration(Duration::from_secs(45)), "45.00s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m 1s");
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1234567890), "1,234,567,890");
    }

    #[test]
    fn test_format_rate() {
        assert_eq!(format_rate(0), "0/sec");
        assert_eq!(format_rate(1234), "1,234/sec");
    }

    #[test]
    fn test_calculate_rate() {
        assert_eq!(calculate_rate(1000, Duration::from_secs(1)), 1000);
        assert_eq!(calculate_rate(0, Duration::from_secs(10)), 0);
        assert_eq!(calculate_rate(5000, Duration::from_secs(5)), 1000);
    }
}
