//! TUI application state management.

use std::collections::HashMap;
use std::time::Instant;

use ratatui::Frame;

use crate::backend::{ChannelMessage, ConsoleEvent};

use super::layout::{calculate_channel_split, calculate_layout};
use super::widgets::{
    render_channel, render_header, render_overview, render_thread_activity, OverviewState,
    ThreadState,
};

/// TUI application state.
///
/// Manages all state for the terminal UI including active jobs, solver states,
/// channel buffers, and scroll positions.
pub struct TuiApp {
    /// All console events received
    events: Vec<ConsoleEvent>,
    /// Events grouped by channel name
    channel_events: HashMap<String, Vec<ConsoleEvent>>,
    /// Scroll offset for log viewing
    scroll_offset: usize,
    /// Active jobs (job_id -> solver_id)
    active_jobs: HashMap<String, u64>,
    /// Solver start time (for elapsed calculation)
    start_time: Instant,
    /// Best score seen so far
    best_score: Option<String>,
    /// Thread states (thread_id -> state)
    threads: HashMap<String, ThreadState>,
}

impl TuiApp {
    /// Creates a new TUI application state.
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            channel_events: HashMap::new(),
            scroll_offset: 0,
            active_jobs: HashMap::new(),
            start_time: Instant::now(),
            best_score: None,
            threads: HashMap::new(),
        }
    }

    /// Handles a console event from a solver thread.
    pub fn handle_console_event(&mut self, event: ConsoleEvent) {
        // Track active jobs
        self.active_jobs
            .entry(event.job_id.clone())
            .or_insert(event.solver_id);

        // Update best score if present
        if let ChannelMessage::Metric { key, value, .. } = &event.message {
            if key == "best_score" {
                self.best_score = Some(value.clone());
            }
        }

        // Update thread state
        let thread_id = match &event.message {
            ChannelMessage::Log { thread_id, .. }
            | ChannelMessage::Metric { thread_id, .. }
            | ChannelMessage::Progress { thread_id, .. } => format!("{:?}", thread_id),
        };

        self.threads
            .entry(thread_id.clone())
            .or_insert_with(|| ThreadState {
                thread_id: thread_id.clone(),
                phase: "Active".to_string(),
                progress: 0.0,
                phase_elapsed: std::time::Duration::from_secs(0),
            });

        // Update progress if available
        if let ChannelMessage::Progress { current, total, .. } = &event.message {
            if let Some(thread_state) = self.threads.get_mut(&thread_id) {
                thread_state.progress = *current as f64 / *total as f64;
                thread_state.phase_elapsed = event.timestamp.duration_since(self.start_time);
            }
        }

        // Store event in channel-specific buffer
        self.channel_events
            .entry(event.channel_name.clone())
            .or_insert_with(Vec::new)
            .push(event.clone());

        // Store event in global buffer
        self.events.push(event);

        // Limit event buffer to last 1000 events
        if self.events.len() > 1000 {
            self.events.drain(0..100);
        }

        // Limit per-channel buffers
        for channel_events in self.channel_events.values_mut() {
            if channel_events.len() > 500 {
                channel_events.drain(0..50);
            }
        }
    }

    /// Renders the UI to the terminal.
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Calculate layout
        let thread_count = self.threads.len();
        let panels = calculate_layout(area, thread_count);

        // Render header
        let job_id = self.active_jobs.keys().next().map(|s| s.as_str());
        render_header(frame, panels.header, job_id);

        // Render overview
        let overview_state = self.build_overview_state();
        render_overview(frame, panels.overview, &overview_state);

        // Render thread activity
        let thread_states: Vec<ThreadState> = self.threads.values().cloned().collect();
        render_thread_activity(frame, panels.thread_activity, &thread_states);

        // Render channels
        let channel_names: Vec<String> = self.channel_events.keys().cloned().collect();
        let channel_count = channel_names.len().max(1);
        let channel_areas = calculate_channel_split(panels.channels, channel_count);

        for (i, channel_name) in channel_names.iter().enumerate() {
            if i < channel_areas.len() {
                if let Some(events) = self.channel_events.get(channel_name) {
                    render_channel(
                        frame,
                        channel_areas[i],
                        channel_name,
                        events,
                        self.scroll_offset,
                    );
                }
            }
        }

        // If no channels, render a placeholder
        if channel_names.is_empty() {
            render_channel(
                frame,
                panels.channels,
                "core",
                &[],
                self.scroll_offset,
            );
        }
    }

    /// Builds overview state from current app state.
    fn build_overview_state(&self) -> OverviewState {
        let (job_id, solver_id) = self
            .active_jobs
            .iter()
            .next()
            .map(|(j, s)| (j.clone(), *s))
            .unwrap_or_else(|| ("N/A".to_string(), 0));

        OverviewState {
            job_id,
            solver_id,
            status: if !self.threads.is_empty() {
                "Solving".to_string()
            } else {
                "Idle".to_string()
            },
            best_score: self
                .best_score
                .clone()
                .unwrap_or_else(|| "N/A".to_string()),
            elapsed: self.start_time.elapsed(),
        }
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
