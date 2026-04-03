// Banner and version display for the solver console.

use owo_colors::OwoColorize;
use std::io::{self, Write};

// Package version for banner display.
const VERSION: &str = env!("CARGO_PKG_VERSION");
const EMERALD: (u8, u8, u8) = (16, 185, 129);

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
    let _ = writeln!(
        stdout,
        "{}",
        banner.truecolor(
            EMERALD.0,
            EMERALD.1,
            EMERALD.2
        )
    );
    let _ = writeln!(
        stdout,
        "{}",
        version_line
            .truecolor(
                EMERALD.0,
                EMERALD.1,
                EMERALD.2
            )
            .bold()
    );
    let _ = stdout.flush();
}
