//! TUI application state management.

use std::collections::HashMap;

use ratatui::Frame;

use crate::backend::ConsoleEvent;

/// TUI application state.
///
/// Manages all state for the terminal UI including active jobs, solver states,
/// channel buffers, and scroll positions.
pub struct TuiApp {
    /// Active console events grouped by job_id
    events: Vec<ConsoleEvent>,
    /// Scroll offset for log viewing
    scroll_offset: usize,
    /// Active jobs (job_id -> solver_id)
    active_jobs: HashMap<String, u64>,
}

impl TuiApp {
    /// Creates a new TUI application state.
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            scroll_offset: 0,
            active_jobs: HashMap::new(),
        }
    }

    /// Handles a console event from a solver thread.
    pub fn handle_console_event(&mut self, event: ConsoleEvent) {
        // Track active jobs
        self.active_jobs
            .entry(event.job_id.clone())
            .or_insert(event.solver_id);

        // Store event
        self.events.push(event);

        // Limit event buffer to last 1000 events
        if self.events.len() > 1000 {
            self.events.drain(0..100);
        }
    }

    /// Renders the UI to the terminal.
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // TODO: Implement actual layout and widgets
        use ratatui::text::Text;
        use ratatui::widgets::{Block, Borders, Paragraph};

        let block = Block::default()
            .title("SERIO Console v0.4.0")
            .borders(Borders::ALL);

        let text = Text::raw(format!(
            "Events: {}\nActive jobs: {}\nScroll: {}",
            self.events.len(),
            self.active_jobs.len(),
            self.scroll_offset
        ));

        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
    }

    /// Scrolls up in the log view.
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scrolls down in the log view.
    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    /// Scrolls up one page.
    pub fn page_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(10);
    }

    /// Scrolls down one page.
    pub fn page_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(10);
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}
