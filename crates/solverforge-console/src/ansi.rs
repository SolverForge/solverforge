//! ANSI color code constants for terminal output.
//!
//! This module provides ANSI escape sequences for coloring terminal output.
//! We implement our own color codes to avoid external dependencies and maintain
//! zero-overhead guarantees.

/// Reset all attributes.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::RESET;
///
/// let colored = format!("{}Red text{}", "\x1b[31m", RESET);
/// assert!(colored.contains("Red text"));
/// ```
pub const RESET: &str = "\x1b[0m";

/// Bold text.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::BOLD;
///
/// let bold_text = format!("{}Bold{}", BOLD, "\x1b[0m");
/// assert!(bold_text.starts_with("\x1b[1m"));
/// ```
pub const BOLD: &str = "\x1b[1m";

/// Red foreground color.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::{RED, RESET};
///
/// let error = format!("{}ERROR{}", RED, RESET);
/// assert!(error.contains("ERROR"));
/// ```
pub const RED: &str = "\x1b[31m";

/// Green foreground color.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::{GREEN, RESET};
///
/// let success = format!("{}OK{}", GREEN, RESET);
/// assert!(success.contains("OK"));
/// ```
pub const GREEN: &str = "\x1b[32m";

/// Yellow foreground color.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::{YELLOW, RESET};
///
/// let warning = format!("{}WARN{}", YELLOW, RESET);
/// assert!(warning.contains("WARN"));
/// ```
pub const YELLOW: &str = "\x1b[33m";

/// Blue foreground color.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::{BLUE, RESET};
///
/// let info = format!("{}INFO{}", BLUE, RESET);
/// assert!(info.contains("INFO"));
/// ```
pub const BLUE: &str = "\x1b[34m";

/// Magenta foreground color.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::{MAGENTA, RESET};
///
/// let metric = format!("{}METRIC{}", MAGENTA, RESET);
/// assert!(metric.contains("METRIC"));
/// ```
pub const MAGENTA: &str = "\x1b[35m";

/// Cyan foreground color.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::{CYAN, RESET};
///
/// let channel = format!("{}[core]{}", CYAN, RESET);
/// assert!(channel.contains("[core]"));
/// ```
pub const CYAN: &str = "\x1b[36m";

/// Bright black (gray) foreground color.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::{BRIGHT_BLACK, RESET};
///
/// let timestamp = format!("{}12:34:56{}", BRIGHT_BLACK, RESET);
/// assert!(timestamp.contains("12:34:56"));
/// ```
pub const BRIGHT_BLACK: &str = "\x1b[90m";

/// Bright red foreground color.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::{BRIGHT_RED, RESET};
///
/// let error = format!("{}FATAL{}", BRIGHT_RED, RESET);
/// assert!(error.contains("FATAL"));
/// ```
pub const BRIGHT_RED: &str = "\x1b[91m";

/// Bright green foreground color.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::{BRIGHT_GREEN, RESET};
///
/// let success = format!("{}SUCCESS{}", BRIGHT_GREEN, RESET);
/// assert!(success.contains("SUCCESS"));
/// ```
pub const BRIGHT_GREEN: &str = "\x1b[92m";

/// Bright yellow foreground color.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::{BRIGHT_YELLOW, RESET};
///
/// let highlight = format!("{}IMPORTANT{}", BRIGHT_YELLOW, RESET);
/// assert!(highlight.contains("IMPORTANT"));
/// ```
pub const BRIGHT_YELLOW: &str = "\x1b[93m";

/// Bright blue foreground color.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::{BRIGHT_BLUE, RESET};
///
/// let progress = format!("{}PROGRESS{}", BRIGHT_BLUE, RESET);
/// assert!(progress.contains("PROGRESS"));
/// ```
pub const BRIGHT_BLUE: &str = "\x1b[94m";

/// Bright magenta foreground color.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::{BRIGHT_MAGENTA, RESET};
///
/// let metric = format!("{}METRIC{}", BRIGHT_MAGENTA, RESET);
/// assert!(metric.contains("METRIC"));
/// ```
pub const BRIGHT_MAGENTA: &str = "\x1b[95m";

/// Bright cyan foreground color.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::{BRIGHT_CYAN, RESET};
///
/// let channel = format!("{}[channel]{}", BRIGHT_CYAN, RESET);
/// assert!(channel.contains("[channel]"));
/// ```
pub const BRIGHT_CYAN: &str = "\x1b[96m";

/// Bright white foreground color.
///
/// # Examples
///
/// ```
/// use solverforge_console::ansi::{BRIGHT_WHITE, RESET};
///
/// let value = format!("{}1234{}", BRIGHT_WHITE, RESET);
/// assert!(value.contains("1234"));
/// ```
pub const BRIGHT_WHITE: &str = "\x1b[97m";
