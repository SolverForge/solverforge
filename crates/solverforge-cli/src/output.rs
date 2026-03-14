use owo_colors::OwoColorize;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Instant;

// Verbosity: 0 = quiet, 1 = normal, 2 = verbose
static VERBOSITY: AtomicU8 = AtomicU8::new(1);

static NO_COLOR: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_verbosity(level: u8) {
    VERBOSITY.store(level, Ordering::Relaxed);
}

pub fn verbosity() -> u8 {
    VERBOSITY.load(Ordering::Relaxed)
}

pub fn set_no_color(enabled: bool) {
    NO_COLOR.store(enabled, Ordering::Relaxed);
}

pub fn is_no_color() -> bool {
    NO_COLOR.load(Ordering::Relaxed)
}

pub fn is_quiet() -> bool {
    verbosity() == 0
}

#[allow(dead_code)]
pub fn is_verbose() -> bool {
    verbosity() >= 2
}

// Applies color only when NO_COLOR is not set
fn colorize_green(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        format!("{}", s.bright_green())
    }
}

fn colorize_cyan(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        format!("{}", s.bright_cyan())
    }
}

fn colorize_white_bold(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        format!("{}", s.bright_white().bold())
    }
}

fn colorize_red(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        format!("{}", s.bright_red())
    }
}

fn colorize_dim(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        format!("{}", s.bright_black())
    }
}

fn colorize_green_bold(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        format!("{}", s.bright_green().bold())
    }
}

fn colorize_yellow(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        format!("{}", s.yellow())
    }
}

// Consistent output verbs (Rails-style)
pub fn print_create(path: &str) {
    if !is_quiet() {
        println!("      {} {}", colorize_green("create"), colorize_cyan(path));
    }
}

pub fn print_update(path: &str) {
    if !is_quiet() {
        println!("      {} {}", colorize_green("update"), colorize_cyan(path));
    }
}

pub fn print_remove(path: &str) {
    if !is_quiet() {
        println!("      {} {}", colorize_green("remove"), colorize_cyan(path));
    }
}

pub fn print_invoke(label: &str) {
    if !is_quiet() {
        println!(
            "      {} {}",
            colorize_green("invoke"),
            colorize_cyan(label)
        );
    }
}

pub fn print_skip(path: &str) {
    if !is_quiet() {
        println!(
            "        {} {}",
            colorize_yellow("skip"),
            colorize_cyan(path)
        );
    }
}

#[allow(dead_code)]
pub fn print_identical(path: &str) {
    if !is_quiet() {
        println!("  {} {}", colorize_yellow("identical"), colorize_cyan(path));
    }
}

pub fn print_status(verb: &str, message: &str) {
    if !is_quiet() {
        println!("      {} {}", colorize_green(verb), message);
    }
}

pub fn print_error(message: &str) {
    eprintln!("{}: {}", colorize_red("error"), message);
}

#[allow(dead_code)]
pub fn print_error_with_hint(message: &str, hint: &str) {
    eprintln!("{}: {}", colorize_red("error"), message);
    eprintln!();
    eprintln!("  {}: {}", colorize_yellow("hint"), hint);
}

pub fn print_success(message: &str) {
    if !is_quiet() {
        println!("{}", colorize_green_bold(message));
    }
}

pub fn print_heading(message: &str) {
    if !is_quiet() {
        println!("{}", colorize_white_bold(message));
    }
}

pub fn print_dim(message: &str) {
    if !is_quiet() {
        println!("{}", colorize_dim(message));
    }
}

#[allow(dead_code)]
pub fn print_verbose(message: &str) {
    if is_verbose() {
        println!("{}", colorize_dim(message));
    }
}

pub fn format_elapsed(start: Instant) -> String {
    let elapsed = start.elapsed();
    if elapsed.as_secs() >= 1 {
        format!("{:.1}s", elapsed.as_secs_f64())
    } else {
        format!("{}ms", elapsed.as_millis())
    }
}
