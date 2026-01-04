//! Message-passing backend for thread-safe console communication.

use std::sync::mpsc;
use std::thread::ThreadId;
use std::time::Instant;
use uuid::Uuid;

/// Message sent from a channel to the console backend.
#[derive(Debug, Clone)]
pub enum ChannelMessage {
    /// Log message with level and content.
    Log {
        /// Thread that generated the message.
        thread_id: ThreadId,
        /// Log level.
        level: crate::channel::LogLevel,
        /// Message content.
        message: String,
    },
    /// Metric update.
    Metric {
        /// Thread that generated the metric.
        thread_id: ThreadId,
        /// Metric key/name.
        key: String,
        /// Metric value.
        value: String,
    },
    /// Progress update.
    Progress {
        /// Thread that generated the progress update.
        thread_id: ThreadId,
        /// Current progress value.
        current: u64,
        /// Total/target value.
        total: u64,
        /// Progress message.
        message: String,
    },
}

/// Event sent to the console backend, including correlation metadata.
#[derive(Debug, Clone)]
pub struct ConsoleEvent {
    /// Job identifier (user-provided).
    pub job_id: String,
    /// Solver instance identifier (auto-generated UUID).
    pub solver_id: Uuid,
    /// Channel name that generated the event.
    pub channel_name: String,
    /// The actual channel message.
    pub message: ChannelMessage,
    /// Timestamp when event was created.
    pub timestamp: Instant,
}

/// Backend for console message passing.
///
/// Provides thread-safe MPSC communication between solver threads and console output.
/// The backend only stores the sender (which is Clone + Send), while the receiver
/// is returned separately to avoid Send/Sync issues.
#[derive(Debug, Clone)]
pub struct ConsoleBackend {
    sender: mpsc::Sender<ConsoleEvent>,
}

impl ConsoleBackend {
    /// Creates a new console backend with a buffered channel.
    ///
    /// Returns the backend (with sender) and the receiver.
    ///
    /// # Examples
    ///
    /// ```
    /// use solverforge_console::backend::ConsoleBackend;
    ///
    /// let (backend, receiver) = ConsoleBackend::new();
    /// ```
    pub fn new() -> (Self, mpsc::Receiver<ConsoleEvent>) {
        let (sender, receiver) = mpsc::channel();
        (Self { sender }, receiver)
    }

    /// Returns a clone of the sender for message submission.
    pub fn sender(&self) -> mpsc::Sender<ConsoleEvent> {
        self.sender.clone()
    }
}
