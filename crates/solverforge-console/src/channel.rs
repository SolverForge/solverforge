//! Output channel with automatic thread tagging.

use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use crate::backend::{ChannelMessage, ConsoleEvent};

/// Log level for channel messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Debug-level message (verbose).
    Debug,
    /// Informational message.
    Info,
    /// Warning message.
    Warn,
    /// Error message.
    Error,
}

/// Output channel for console messages.
///
/// Channels provide isolated output streams with automatic thread ID tagging.
/// All messages sent through a channel are non-blocking (buffered MPSC).
///
/// # Examples
///
/// ```
/// use solverforge_console::channel::Channel;
/// use std::sync::mpsc;
///
/// let (sender, _receiver) = mpsc::channel();
/// let channel = Channel::new(
///     "myapp".to_string(),
///     "job-001".to_string(),
///     1,
///     sender,
/// );
///
/// // Log messages with automatic thread tagging
/// channel.info("Application started");
/// channel.warn("High memory usage detected");
/// ```
#[derive(Debug, Clone)]
pub struct Channel {
    name: String,
    job_id: String,
    solver_id: u64,
    sender: mpsc::Sender<ConsoleEvent>,
}

impl Channel {
    /// Creates a new channel.
    ///
    /// # Arguments
    ///
    /// * `name` - Channel name (e.g., "core", "myapp")
    /// * `job_id` - Job identifier for correlation
    /// * `solver_id` - Solver instance identifier (sequential ID)
    /// * `sender` - MPSC sender to console backend
    pub fn new(
        name: String,
        job_id: String,
        solver_id: u64,
        sender: mpsc::Sender<ConsoleEvent>,
    ) -> Self {
        Self {
            name,
            job_id,
            solver_id,
            sender,
        }
    }

    /// Returns the channel name.
    ///
    /// # Examples
    ///
    /// ```
    /// # use solverforge_console::channel::Channel;
    /// # use std::sync::mpsc;
    /// # let (sender, _) = mpsc::channel();
    /// # let channel = Channel::new("test".to_string(), "job".to_string(), 1, sender);
    /// assert_eq!(channel.name(), "test");
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Logs a message at the specified level.
    ///
    /// Automatically captures the current thread ID.
    ///
    /// # Examples
    ///
    /// ```
    /// # use solverforge_console::channel::{Channel, LogLevel};
    /// # use std::sync::mpsc;
    /// # let (sender, _) = mpsc::channel();
    /// # let channel = Channel::new("test".to_string(), "job".to_string(), 1, sender);
    /// channel.log(LogLevel::Info, "Solver initialized".to_string());
    /// ```
    pub fn log(&self, level: LogLevel, message: String) {
        let event = ConsoleEvent {
            job_id: self.job_id.clone(),
            solver_id: self.solver_id,
            channel_name: self.name.clone(),
            message: ChannelMessage::Log {
                thread_id: thread::current().id(),
                level,
                message,
            },
            timestamp: Instant::now(),
        };

        let _ = self.sender.send(event);
    }

    /// Logs a debug-level message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use solverforge_console::channel::Channel;
    /// # use std::sync::mpsc;
    /// # let (sender, _) = mpsc::channel();
    /// # let channel = Channel::new("test".to_string(), "job".to_string(), 1, sender);
    /// channel.debug("Entering function foo()");
    /// ```
    pub fn debug(&self, message: impl Into<String>) {
        self.log(LogLevel::Debug, message.into());
    }

    /// Logs an info-level message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use solverforge_console::channel::Channel;
    /// # use std::sync::mpsc;
    /// # let (sender, _) = mpsc::channel();
    /// # let channel = Channel::new("test".to_string(), "job".to_string(), 1, sender);
    /// channel.info("Problem loaded successfully");
    /// ```
    pub fn info(&self, message: impl Into<String>) {
        self.log(LogLevel::Info, message.into());
    }

    /// Logs a warning-level message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use solverforge_console::channel::Channel;
    /// # use std::sync::mpsc;
    /// # let (sender, _) = mpsc::channel();
    /// # let channel = Channel::new("test".to_string(), "job".to_string(), 1, sender);
    /// channel.warn("Memory usage above 80%");
    /// ```
    pub fn warn(&self, message: impl Into<String>) {
        self.log(LogLevel::Warn, message.into());
    }

    /// Logs an error-level message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use solverforge_console::channel::Channel;
    /// # use std::sync::mpsc;
    /// # let (sender, _) = mpsc::channel();
    /// # let channel = Channel::new("test".to_string(), "job".to_string(), 1, sender);
    /// channel.error("Failed to load constraint data");
    /// ```
    pub fn error(&self, message: impl Into<String>) {
        self.log(LogLevel::Error, message.into());
    }

    /// Sends a metric update.
    ///
    /// # Examples
    ///
    /// ```
    /// # use solverforge_console::channel::Channel;
    /// # use std::sync::mpsc;
    /// # let (sender, _) = mpsc::channel();
    /// # let channel = Channel::new("test".to_string(), "job".to_string(), 1, sender);
    /// channel.metric("moves_evaluated", "123456");
    /// channel.metric("best_score", "0hard/-1234soft");
    /// ```
    pub fn metric(&self, key: impl Into<String>, value: impl Into<String>) {
        let event = ConsoleEvent {
            job_id: self.job_id.clone(),
            solver_id: self.solver_id,
            channel_name: self.name.clone(),
            message: ChannelMessage::Metric {
                thread_id: thread::current().id(),
                key: key.into(),
                value: value.into(),
            },
            timestamp: Instant::now(),
        };

        let _ = self.sender.send(event);
    }

    /// Sends a progress update.
    ///
    /// # Examples
    ///
    /// ```
    /// # use solverforge_console::channel::Channel;
    /// # use std::sync::mpsc;
    /// # let (sender, _) = mpsc::channel();
    /// # let channel = Channel::new("test".to_string(), "job".to_string(), 1, sender);
    /// // Report progress: 500 out of 1000 steps completed
    /// channel.progress(500, 1000, "Constructing initial solution");
    /// ```
    pub fn progress(&self, current: u64, total: u64, message: impl Into<String>) {
        let event = ConsoleEvent {
            job_id: self.job_id.clone(),
            solver_id: self.solver_id,
            channel_name: self.name.clone(),
            message: ChannelMessage::Progress {
                thread_id: thread::current().id(),
                current,
                total,
                message: message.into(),
            },
            timestamp: Instant::now(),
        };

        let _ = self.sender.send(event);
    }

    /// Sends a solver status update.
    ///
    /// # Examples
    ///
    /// ```
    /// # use solverforge_console::channel::Channel;
    /// # use solverforge_console::backend::SolverState;
    /// # use std::sync::mpsc;
    /// # let (sender, _) = mpsc::channel();
    /// # let channel = Channel::new("test".to_string(), "job".to_string(), 1, sender);
    /// // Report that solving has started
    /// channel.status(SolverState::Solving);
    ///
    /// // Later, report completion
    /// channel.status(SolverState::Completed);
    /// ```
    pub fn status(&self, status: crate::backend::SolverState) {
        let event = ConsoleEvent {
            job_id: self.job_id.clone(),
            solver_id: self.solver_id,
            channel_name: self.name.clone(),
            message: ChannelMessage::SolverStatus {
                thread_id: thread::current().id(),
                status,
            },
            timestamp: Instant::now(),
        };

        let _ = self.sender.send(event);
    }
}
