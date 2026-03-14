use std::fmt;

pub type CliResult<T = ()> = Result<T, CliError>;

pub enum CliError {
    // Project scaffolding errors
    DirectoryExists {
        name: String,
    },
    InvalidProjectName {
        name: String,
        reason: &'static str,
    },
    ReservedKeyword {
        name: String,
    },

    // "Not in a project" errors
    NotInProject {
        missing: &'static str,
    },

    // Resource errors
    ResourceExists {
        kind: &'static str,
        name: String,
    },
    ResourceNotFound {
        kind: &'static str,
        name: String,
    },

    // Validation
    InvalidName {
        name: String,
    },
    InvalidScoreType {
        score: String,
        known: &'static [&'static str],
    },

    // IO and subprocess
    IoError {
        context: String,
        source: std::io::Error,
    },
    SubprocessFailed {
        command: String,
    },

    // General with optional hint
    General {
        message: String,
        hint: Option<String>,
    },
}

impl CliError {
    pub fn general(message: impl Into<String>) -> Self {
        CliError::General {
            message: message.into(),
            hint: None,
        }
    }

    pub fn with_hint(message: impl Into<String>, hint: impl Into<String>) -> Self {
        CliError::General {
            message: message.into(),
            hint: Some(hint.into()),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::DirectoryExists { name } => {
                write!(f, "directory '{}' already exists", name)
            }
            CliError::InvalidProjectName { name, reason } => {
                write!(f, "invalid project name '{}': {}", name, reason)
            }
            CliError::ReservedKeyword { name } => {
                write!(
                    f,
                    "'{}' is a Rust reserved keyword and cannot be used as a project name",
                    name
                )
            }
            CliError::NotInProject { missing } => {
                write!(
                    f,
                    "not a SolverForge project directory ({} not found)\n\n  \
                     hint: run `solverforge new <name> --basic` to create a project,\n  \
                     or `cd` into an existing SolverForge project directory",
                    missing
                )
            }
            CliError::ResourceExists { kind, name } => {
                write!(f, "{} '{}' already exists", kind, name)
            }
            CliError::ResourceNotFound { kind, name } => {
                write!(f, "{} '{}' not found", kind, name)
            }
            CliError::InvalidName { name } => {
                write!(
                    f,
                    "invalid name '{}': use snake_case (lowercase letters, digits, underscores; must start with a letter)",
                    name
                )
            }
            CliError::InvalidScoreType { score, known } => {
                write!(
                    f,
                    "unknown score type '{}'\n\nKnown score types: {}",
                    score,
                    known.join(", ")
                )
            }
            CliError::IoError { context, source } => {
                write!(f, "{}: {}", context, source)
            }
            CliError::SubprocessFailed { command } => {
                write!(f, "'{}' failed", command)
            }
            CliError::General { message, hint } => {
                write!(f, "{}", message)?;
                if let Some(h) = hint {
                    write!(f, "\n\n  hint: {}", h)?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Debug for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<String> for CliError {
    fn from(s: String) -> Self {
        CliError::General {
            message: s,
            hint: None,
        }
    }
}

// Rust reserved keywords that cannot be used as crate names
const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "union",
    "unsafe", "use", "where", "while", "yield", "abstract", "become", "box", "do", "final",
    "macro", "override", "priv", "try", "typeof", "unsized", "virtual",
];

pub fn is_rust_keyword(name: &str) -> bool {
    RUST_KEYWORDS.contains(&name)
}
