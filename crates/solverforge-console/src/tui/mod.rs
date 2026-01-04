//! Terminal User Interface (TUI) for SERIO Console.
//!
//! This module implements the rich interactive terminal UI using ratatui.
//! The TUI runs in a separate thread and provides real-time visualization of
//! solver progress, metrics, and multi-channel output.

mod app;
mod events;
pub mod layout;
mod widgets;

use std::io;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::backend::ConsoleEvent;
use app::TuiApp;

/// Runs the TUI event loop (blocking).
///
/// This function sets up the terminal, runs the event loop at 60 FPS,
/// and handles keyboard input and console events.
///
/// # Arguments
///
/// * `receiver` - MPSC receiver for console events from solver threads
///
/// # Examples
///
/// ```no_run
/// use solverforge_console::tui::run_tui;
/// use solverforge_console::backend::ConsoleBackend;
///
/// let (backend, receiver) = ConsoleBackend::new();
///
/// // Run in separate thread
/// std::thread::spawn(move || {
///     run_tui(receiver).ok();
/// });
/// ```
pub fn run_tui(receiver: mpsc::Receiver<ConsoleEvent>) -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = TuiApp::new();

    // Run event loop at 60 FPS
    let tick_rate = Duration::from_millis(16); // ~60 FPS
    let mut should_quit = false;

    while !should_quit {
        // Draw UI
        terminal.draw(|f| {
            app.render(f);
        })?;

        // Handle keyboard events (non-blocking with timeout)
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                should_quit = handle_key_event(key, &mut app);
            }
        }

        // Process console events (non-blocking)
        while let Ok(event) = receiver.try_recv() {
            app.handle_console_event(event);
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

/// Handles keyboard input events.
///
/// Returns true if the application should quit.
fn handle_key_event(key: KeyEvent, app: &mut TuiApp) -> bool {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => true,
        KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => true,
        KeyCode::Up => {
            app.scroll_up();
            false
        }
        KeyCode::Down => {
            app.scroll_down();
            false
        }
        KeyCode::PageUp => {
            app.page_up();
            false
        }
        KeyCode::PageDown => {
            app.page_down();
            false
        }
        _ => false,
    }
}
