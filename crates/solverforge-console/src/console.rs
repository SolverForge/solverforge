//! Console management and coordination.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, OnceLock};
use parking_lot::Mutex;

use crate::backend::{ConsoleBackend, ConsoleEvent};
use crate::channel::Channel;

/// Global atomic counter for generating unique solver IDs.
///
/// This counter is incremented atomically for each new ConsoleInstance,
/// providing lock-free sequential solver IDs.
static SOLVER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Global console manager singleton.
static CONSOLE_MANAGER: OnceLock<ConsoleManager> = OnceLock::new();

/// Console output mode.
///
/// Currently only TUI mode is supported. This enum exists for potential
/// future expansion but for now only provides the rich terminal UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleMode {
    /// Rich terminal UI with live updates (ratatui-based).
    Tui,
}

/// Global console manager (singleton).
///
/// The console manager is a global singleton that coordinates all console instances
/// and runs the TUI backend. Users initialize it once at startup and access it from
/// anywhere in the code.
///
/// # Usage Pattern
///
/// 1. **Wire once at startup**: Call `ConsoleManager::init()` in main
/// 2. **Run TUI thread**: Spawn thread calling `ConsoleManager::run()`
/// 3. **Use anywhere**: Call `ConsoleManager::global().create_console(job_id)` to get instances
///
/// # Examples
///
/// ```no_run
/// use solverforge_console::{ConsoleManager, ConsoleMode};
///
/// // In main.rs - wire once
/// ConsoleManager::init(ConsoleMode::Tui);
///
/// // Start TUI in background thread
/// std::thread::spawn(|| {
///     ConsoleManager::run();
/// });
///
/// // Anywhere in code - just use it
/// let mut console = ConsoleManager::global().create_console("job-1".to_string());
/// let core = console.core_channel();
/// core.info("Solver starting");
/// ```
#[derive(Debug)]
pub struct ConsoleManager {
    backend: Arc<ConsoleBackend>,
    receiver: Arc<Mutex<Option<mpsc::Receiver<ConsoleEvent>>>>,
    mode: ConsoleMode,
}

impl ConsoleManager {
    /// Initializes the global console manager.
    ///
    /// Must be called once at startup before using the console system.
    ///
    /// # Panics
    ///
    /// Panics if called more than once.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use solverforge_console::{ConsoleManager, ConsoleMode};
    ///
    /// // In main.rs - call once at startup
    /// ConsoleManager::init(ConsoleMode::Tui);
    /// ```
    pub fn init(mode: ConsoleMode) {
        let (backend, receiver) = ConsoleBackend::new();
        let manager = Self {
            backend: Arc::new(backend),
            receiver: Arc::new(Mutex::new(Some(receiver))),
            mode,
        };
        CONSOLE_MANAGER.set(manager)
            .expect("ConsoleManager::init() called more than once");
    }

    /// Returns the global console manager instance.
    ///
    /// # Panics
    ///
    /// Panics if `init()` has not been called.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use solverforge_console::ConsoleManager;
    ///
    /// // Access global instance from anywhere
    /// let console = ConsoleManager::global().create_console("job-1".to_string());
    /// ```
    pub fn global() -> &'static ConsoleManager {
        CONSOLE_MANAGER.get()
            .expect("ConsoleManager not initialized - call ConsoleManager::init() first")
    }

    /// Creates a console instance for a specific job.
    ///
    /// # Arguments
    ///
    /// * `job_id` - User-provided job identifier for correlation
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use solverforge_console::ConsoleManager;
    ///
    /// let console = ConsoleManager::global().create_console("vrp-001".to_string());
    /// ```
    pub fn create_console(&self, job_id: String) -> ConsoleInstance {
        ConsoleInstance::new(job_id, self.backend.clone())
    }

    /// Runs the console backend (blocking).
    ///
    /// This method blocks and runs the TUI event loop. Must be run in a
    /// separate thread to avoid blocking the solver.
    ///
    /// # Panics
    ///
    /// Panics if `init()` has not been called or if `run()` is called more than once.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use solverforge_console::{ConsoleManager, ConsoleMode};
    ///
    /// ConsoleManager::init(ConsoleMode::Tui);
    ///
    /// // Run TUI in background thread
    /// let handle = std::thread::spawn(|| {
    ///     ConsoleManager::run();
    /// });
    ///
    /// // ... use console elsewhere ...
    ///
    /// handle.join().ok();
    /// ```
    pub fn run() {
        let receiver = Self::global()
            .receiver
            .lock()
            .take()
            .expect("ConsoleManager::run() called more than once");

        // Run TUI event loop
        crate::tui::run_tui(receiver).expect("Failed to run TUI");
    }
}

/// Per-solver console instance.
///
/// Each console instance represents a single solver run with a unique solver ID
/// and associated job ID. It manages multiple output channels.
///
/// # Examples
///
/// ```no_run
/// use solverforge_console::ConsoleManager;
///
/// let mut console = ConsoleManager::global().create_console("vrp-001".to_string());
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
    backend: Arc<ConsoleBackend>,
    channels: HashMap<String, Channel>,
}

impl ConsoleInstance {
    /// Creates a new console instance.
    fn new(job_id: String, backend: Arc<ConsoleBackend>) -> Self {
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
    /// ```no_run
    /// # use solverforge_console::{ConsoleManager, ConsoleMode};
    /// # ConsoleManager::init(ConsoleMode::Tui);
    /// let console = ConsoleManager::global().create_console("my-job".to_string());
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
    /// ```no_run
    /// # use solverforge_console::{ConsoleManager, ConsoleMode};
    /// # ConsoleManager::init(ConsoleMode::Tui);
    /// let console = ConsoleManager::global().create_console("job-1".to_string());
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
    /// ```no_run
    /// # use solverforge_console::{ConsoleManager, ConsoleMode};
    /// # ConsoleManager::init(ConsoleMode::Tui);
    /// # let mut console = ConsoleManager::global().create_console("job".to_string());
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
    /// ```no_run
    /// # use solverforge_console::{ConsoleManager, ConsoleMode};
    /// # ConsoleManager::init(ConsoleMode::Tui);
    /// # let mut console = ConsoleManager::global().create_console("job".to_string());
    /// let core = console.core_channel();
    /// core.info("Solver initialized");
    /// ```
    pub fn core_channel(&mut self) -> &mut Channel {
        self.channel("core")
    }
}
