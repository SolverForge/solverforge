//! SERIO Console - Production-grade console and logging system for SolverForge
//!
//! SERIO Console provides a multi-channel, thread-aware logging and visualization system
//! for SolverForge optimization applications. It supports:
//!
//! - **Multi-channel output**: Separate channels for core solver output and application-specific logs
//! - **Thread correlation**: Automatic thread ID tagging for all events
//! - **Job/Solver correlation**: Track multiple solver instances across distributed systems
//! - **Rich TUI**: Interactive terminal UI with real-time metrics and progress visualization
//! - **Zero-erasure compliance**: Non-blocking message passing, no hot-path overhead
//! - **Production-ready**: Supports orchestrated backends with multiple concurrent jobs
//!
//! # Architecture
//!
//! SERIO Console uses a message-passing architecture:
//!
//! ```text
//! Solver Threads → Channels → ConsoleBackend → TUI/Simple Output
//!      ↓              ↓            ↓
//!   (non-blocking) (MPSC)    (event loop)
//! ```
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use solverforge_console::{ConsoleManager, ConsoleMode};
//!
//! // Create console manager in TUI mode
//! let mut console_mgr = ConsoleManager::new(ConsoleMode::Tui);
//! let mut console = console_mgr.create_console("my-job-001".to_string());
//!
//! // Get core channel (channel 0) for solver output
//! let core = console.core_channel();
//! core.info("Solver initialized");
//!
//! // Create custom application channel
//! let app_channel = console.channel("myapp");
//! app_channel.info("Loading problem data...");
//!
//! // Run TUI in separate thread
//! let tui_handle = std::thread::spawn(move || {
//!     console_mgr.run();
//! });
//!
//! // ... run solver ...
//!
//! // Cleanup
//! # drop(console);
//! # tui_handle.join().ok();
//! ```
//!
//! # Channels
//!
//! Channels provide isolated output streams:
//!
//! - **Channel 0 ("core")**: Reserved for SolverForge framework output
//! - **Named channels**: User-defined channels for application components
//!
//! Each channel automatically tags messages with the current thread ID.
//!
//! # Correlation Tracking
//!
//! SERIO Console provides three-level correlation:
//!
//! - **Job ID**: User-provided string identifying the optimization job
//! - **Solver ID**: Auto-generated UUID for each solver instance
//! - **Thread ID**: Automatically captured from `std::thread::current().id()`
//!
//! This enables tracking: `Job → Solver Instance → Thread → Move/Event`

#![warn(missing_docs)]

pub mod ansi;
pub mod backend;
pub mod channel;
pub mod console;
pub mod event_listener;
pub mod formatter;
pub mod tui;

// Re-exports for convenience
pub use console::{ConsoleInstance, ConsoleManager, ConsoleMode};
pub use channel::{Channel, LogLevel};
pub use backend::{ChannelMessage, ConsoleEvent};
pub use event_listener::ConsoleEventListener;
