//! Console management and coordination.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;

use crate::backend::{ConsoleBackend, ConsoleEvent};
use crate::channel::Channel;

/// Global atomic counter for generating unique solver IDs.
///
/// This counter is incremented atomically for each new ConsoleInstance,
/// providing lock-free sequential solver IDs.
static SOLVER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Console output mode.
///
/// Currently only TUI mode is supported. This enum exists for potential
/// future expansion but for now only provides the rich terminal UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleMode {
    /// Rich terminal UI with live updates (ratatui-based).
    Tui,
}

/// Global console manager.
///
/// The console manager coordinates all console instances and runs the output
/// backend (TUI mode).
///
/// # Examples
///
/// ```
/// use solverforge_console::{ConsoleManager, ConsoleMode};
///
/// // Create manager in TUI mode
/// let mut manager = ConsoleManager::new(ConsoleMode::Tui);
///
/// // Create console for a specific job
/// let console = manager.create_console("vrp-job-001".to_string());
///
/// // Use console channels
/// let mut console_clone = console.clone();
/// let core = console_clone.core_channel();
/// core.info("Solver initialized");
///
/// // Run TUI in separate thread
/// let handle = std::thread::spawn(move || {
///     manager.run();
/// });
///
/// // ... run solver ...
/// # drop(console);
/// # drop(handle);
/// ```
#[derive(Debug)]
pub struct ConsoleManager {
    backend: ConsoleBackend,
    receiver: Option<mpsc::Receiver<ConsoleEvent>>,
    mode: ConsoleMode,
}

impl ConsoleManager {
    /// Creates a new console manager.
    ///
    /// # Examples
    ///
    /// ```
    /// use solverforge_console::{ConsoleManager, ConsoleMode};
    ///
    /// let manager = ConsoleManager::new(ConsoleMode::Tui);
    /// ```
    pub fn new(mode: ConsoleMode) -> Self {
        let (backend, receiver) = ConsoleBackend::new();
        Self {
            backend,
            receiver: Some(receiver),
            mode,
        }
    }

    /// Creates a console instance for a specific job.
    ///
    /// # Arguments
    ///
    /// * `job_id` - User-provided job identifier for correlation
    ///
    /// # Examples
    ///
    /// ```
    /// use solverforge_console::{ConsoleManager, ConsoleMode};
    ///
    /// let mut manager = ConsoleManager::new(ConsoleMode::Tui);
    /// let console = manager.create_console("my-optimization-job".to_string());
    /// ```
    pub fn create_console(&self, job_id: String) -> ConsoleInstance {
        ConsoleInstance::new(job_id, self.backend.clone())
    }

    /// Runs the console backend (blocking).
    ///
    /// This method blocks and runs the TUI event loop. Must be run in a
    /// separate thread to avoid blocking the solver.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use solverforge_console::{ConsoleManager, ConsoleMode};
    ///
    /// let mut manager = ConsoleManager::new(ConsoleMode::Tui);
    /// let console = manager.create_console("job-1".to_string());
    ///
    /// // Run TUI in separate thread
    /// let handle = std::thread::spawn(move || {
    ///     manager.run();
    /// });
    ///
    /// // ... use console ...
    ///
    /// # drop(console);
    /// handle.join().ok();
    /// ```
    pub fn run(mut self) {
        let _receiver = self.receiver.take()
            .expect("ConsoleManager::run called more than once");

        // Run TUI event loop
        // TODO: Implement TUI
        unimplemented!("TUI mode not yet implemented")
    }
}

/// Per-solver console instance.
///
/// Each console instance represents a single solver run with a unique solver ID
/// and associated job ID. It manages multiple output channels.
///
/// # Examples
///
/// ```
/// use solverforge_console::{ConsoleManager, ConsoleMode};
///
/// let mut manager = ConsoleManager::new(ConsoleMode::Tui);
/// let mut console = manager.create_console("vrp-001".to_string());
///
/// // Get core channel (always available)
/// let core = console.core_channel();
/// core.info("Solver starting");
///
/// // Create custom application channel
/// let app = console.channel("myapp");
/// app.info("Loading problem data");
/// ```
#[derive(Debug, Clone)]
pub struct ConsoleInstance {
    job_id: String,
    solver_id: u64,
    backend: ConsoleBackend,
    channels: HashMap<String, Channel>,
}

impl ConsoleInstance {
    /// Creates a new console instance.
    fn new(job_id: String, backend: ConsoleBackend) -> Self {
        let solver_id = SOLVER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut instance = Self {
            job_id: job_id.clone(),
            solver_id,
            backend,
            channels: HashMap::new(),
        };

        // Create core channel (channel 0)
        instance.channel("core");

        instance
    }

    /// Returns the job ID for this console instance.
    ///
    /// # Examples
    ///
    /// ```
    /// # use solverforge_console::{ConsoleManager, ConsoleMode};
    /// # let manager = ConsoleManager::new(ConsoleMode::Tui);
    /// let console = manager.create_console("my-job".to_string());
    /// assert_eq!(console.job_id(), "my-job");
    /// ```
    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    /// Returns the solver ID for this console instance.
    ///
    /// Each solver instance gets a unique sequential ID from an atomic counter.
    ///
    /// # Examples
    ///
    /// ```
    /// # use solverforge_console::{ConsoleManager, ConsoleMode};
    /// # let manager = ConsoleManager::new(ConsoleMode::Tui);
    /// let console = manager.create_console("job-1".to_string());
    /// let solver_id = console.solver_id();
    /// // solver_id is a unique sequential ID (e.g., 1, 2, 3...)
    /// assert!(solver_id > 0);
    /// ```
    pub fn solver_id(&self) -> u64 {
        self.solver_id
    }

    /// Gets or creates a named channel.
    ///
    /// Channels are created on first access. The same channel instance is
    /// returned for subsequent calls with the same name.
    ///
    /// # Examples
    ///
    /// ```
    /// # use solverforge_console::{ConsoleManager, ConsoleMode};
    /// # let manager = ConsoleManager::new(ConsoleMode::Simple);
    /// # let mut console = manager.create_console("job".to_string());
    /// let app_channel = console.channel("myapp");
    /// app_channel.info("Application message");
    ///
    /// // Same channel returned on second call
    /// let same_channel = console.channel("myapp");
    /// ```
    pub fn channel(&mut self, name: &str) -> &mut Channel {
        self.channels.entry(name.to_string()).or_insert_with(|| {
            Channel::new(
                name.to_string(),
                self.job_id.clone(),
                self.solver_id,
                self.backend.sender(),
            )
        })
    }

    /// Returns the core channel (channel 0).
    ///
    /// The core channel is reserved for SolverForge framework output.
    ///
    /// # Examples
    ///
    /// ```
    /// # use solverforge_console::{ConsoleManager, ConsoleMode};
    /// # let manager = ConsoleManager::new(ConsoleMode::Simple);
    /// # let mut console = manager.create_console("job".to_string());
    /// let core = console.core_channel();
    /// core.info("Solver initialized");
    /// ```
    pub fn core_channel(&mut self) -> &mut Channel {
        self.channel("core")
    }
}
