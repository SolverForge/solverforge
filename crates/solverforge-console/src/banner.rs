// Banner and version display for the solver console.

use owo_colors::OwoColorize;
use std::io::{self, Write};

// Package version for banner display.
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn print_banner() {
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
