//! Custom TUI widgets for SERIO Console.
//!
//! Provides specialized widgets for displaying solver state, metrics,
//! thread activity, and multi-channel output.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::backend::{ChannelMessage, ConsoleEvent};
use crate::channel::LogLevel;

/// Renders the header banner.
///
/// Displays the SERIO Console banner with version and active job information.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to render to
/// * `area` - Screen area for the header
/// * `job_id` - Optional active job identifier
pub fn render_header(frame: &mut Frame, area: Rect, job_id: Option<&str>) {
    let job_text = job_id
        .map(|id| format!(" [Job: {}]", id))
        .unwrap_or_default();

    let header_text = format!("SERIO Console v0.4.0{}", job_text);

    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    frame.render_widget(header, area);
}

/// Renders the solver overview panel.
///
/// Displays global metrics: job ID, solver ID, best score, solving time.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to render to
/// * `area` - Screen area for the overview panel
/// * `state` - Solver state information
pub fn render_overview(frame: &mut Frame, area: Rect, state: &OverviewState) {
    let lines = vec![
        Line::from(vec![
            Span::styled("Job: ", Style::default().fg(Color::Gray)),
            Span::raw(&state.job_id),
            Span::raw(" | "),
            Span::styled("Solver: ", Style::default().fg(Color::Gray)),
            Span::raw(state.solver_id.to_string()),
            Span::raw(" | "),
            Span::styled("Status: ", Style::default().fg(Color::Gray)),
            Span::styled(&state.status, Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Best Score: ", Style::default().fg(Color::Gray)),
            Span::styled(&state.best_score, Style::default().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::styled("Time: ", Style::default().fg(Color::Gray)),
            Span::raw(format_duration(state.elapsed)),
        ]),
    ];

    let block = Block::default()
        .title("Solver Overview")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// State for the overview panel.
///
/// # Examples
///
/// ```
/// use solverforge_console::tui::widgets::OverviewState;
/// use std::time::Duration;
///
/// let state = OverviewState {
///     job_id: "vrp-001".to_string(),
///     solver_id: 1,
///     status: "Solving".to_string(),
///     best_score: "0hard/-1234soft".to_string(),
///     elapsed: Duration::from_secs(42),
/// };
///
/// assert_eq!(state.job_id, "vrp-001");
/// assert_eq!(state.solver_id, 1);
/// ```
#[derive(Debug, Clone)]
pub struct OverviewState {
    /// Job identifier.
    pub job_id: String,
    /// Solver instance ID.
    pub solver_id: u64,
    /// Current solver status (e.g., "Solving", "Completed").
    pub status: String,
    /// Best score found so far.
    pub best_score: String,
    /// Elapsed solving time.
    pub elapsed: std::time::Duration,
}

/// Renders the thread activity panel.
///
/// Displays progress indicators for each active solver thread using animated Gauge widgets.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to render to
/// * `area` - Screen area for the thread activity panel
/// * `threads` - Thread activity state
pub fn render_thread_activity(frame: &mut Frame, area: Rect, threads: &[ThreadState]) {
    let block = Block::default()
        .title("Thread Activity")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if threads.is_empty() {
        return;
    }

    // Create vertical layout for each thread (2 lines per thread: label + gauge)
    let constraints: Vec<Constraint> = threads
        .iter()
        .flat_map(|_| vec![Constraint::Length(1), Constraint::Length(1)])
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    // Render each thread with label + gauge
    for (i, thread) in threads.iter().enumerate() {
        let label_area = chunks[i * 2];
        let gauge_area = chunks[i * 2 + 1];

        // Render thread label
        let label = Paragraph::new(format!(
            "Thread-{} │ {} │ {}",
            thread.thread_id,
            thread.phase,
            format_duration(thread.phase_elapsed)
        ))
        .style(Style::default().fg(Color::Gray));
        frame.render_widget(label, label_area);

        // Determine gauge color based on progress (gradient: Red → Yellow → Green → Cyan)
        let progress_percent = (thread.progress * 100.0) as u8;
        let gauge_color = match progress_percent {
            0..=33 => Color::Red,      // Startup/warming up
            34..=66 => Color::Yellow,  // Active solving
            67..=99 => Color::Green,   // Near completion
            _ => Color::Cyan,          // Complete
        };

        // Render animated gauge
        let gauge = Gauge::default()
            .gauge_style(
                Style::default()
                    .fg(gauge_color)
                    .add_modifier(Modifier::BOLD),
            )
            .ratio(thread.progress)
            .label(format!("{:.0}%", thread.progress * 100.0));

        frame.render_widget(gauge, gauge_area);
    }
}

/// State for a single thread.
///
/// # Examples
///
/// ```
/// use solverforge_console::tui::widgets::ThreadState;
/// use std::time::Duration;
///
/// let thread = ThreadState {
///     thread_id: format!("{:?}", std::thread::current().id()),
///     phase: "LocalSearch".to_string(),
///     progress: 0.75,
///     phase_elapsed: Duration::from_secs(10),
/// };
///
/// assert_eq!(thread.progress, 0.75);
/// assert!(thread.progress >= 0.0 && thread.progress <= 1.0);
/// ```
#[derive(Debug, Clone)]
pub struct ThreadState {
    /// Thread identifier (formatted).
    pub thread_id: String,
    /// Current phase name.
    pub phase: String,
    /// Progress ratio (0.0 to 1.0).
    pub progress: f64,
    /// Elapsed time in current phase.
    pub phase_elapsed: std::time::Duration,
}

/// Renders a channel output panel.
///
/// Displays scrollable log messages for a specific channel.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to render to
/// * `area` - Screen area for the channel panel
/// * `channel_name` - Name of the channel
/// * `events` - Channel events to display
/// * `scroll_offset` - Scroll position
pub fn render_channel(
    frame: &mut Frame,
    area: Rect,
    channel_name: &str,
    events: &[ConsoleEvent],
    scroll_offset: usize,
) {
    // Calculate max width for text, accounting for borders (2) and padding (2)
    let max_width = (area.width.saturating_sub(4)) as usize;

    let items: Vec<ListItem> = events
        .iter()
        .skip(scroll_offset)
        .take(area.height.saturating_sub(2) as usize)
        .map(|event| format_event_line(event, max_width))
        .collect();

    let block = Block::default()
        .title(format!("Channel: {}", channel_name))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

/// Formats a console event as a list item with text truncation.
///
/// # Arguments
///
/// * `event` - Console event to format
/// * `max_width` - Maximum width for the entire line (including prefixes)
fn format_event_line(event: &ConsoleEvent, max_width: usize) -> ListItem<'static> {
    match &event.message {
        ChannelMessage::Log { thread_id, level, message } => {
            let level_style = match level {
                LogLevel::Debug => Style::default().fg(Color::Gray),
                LogLevel::Info => Style::default().fg(Color::Cyan),
                LogLevel::Warn => Style::default().fg(Color::Yellow),
                LogLevel::Error => Style::default().fg(Color::Red),
            };

            let level_text = match level {
                LogLevel::Debug => "DEBUG",
                LogLevel::Info => "INFO ",
                LogLevel::Warn => "WARN ",
                LogLevel::Error => "ERROR",
            };

            // Calculate prefix length: "[ThreadId(X)] LEVEL "
            let thread_prefix = format!("[{:?}] ", thread_id);
            let prefix_len = thread_prefix.chars().count() + level_text.chars().count() + 1;

            // Calculate remaining space for message
            let message_max_width = max_width.saturating_sub(prefix_len);
            let truncated_message = truncate_text(message, message_max_width);

            let line = Line::from(vec![
                Span::styled(thread_prefix, Style::default().fg(Color::DarkGray)),
                Span::styled(level_text, level_style),
                Span::raw(" "),
                Span::raw(truncated_message),
            ]);

            ListItem::new(line)
        }
        ChannelMessage::Metric { thread_id, key, value } => {
            let thread_prefix = format!("[{:?}] ", thread_id);
            let prefix_len = thread_prefix.chars().count() + key.chars().count() + 2; // +2 for ": "

            let value_max_width = max_width.saturating_sub(prefix_len);
            let truncated_value = truncate_text(value, value_max_width);

            let line = Line::from(vec![
                Span::styled(thread_prefix, Style::default().fg(Color::DarkGray)),
                Span::styled(key.clone(), Style::default().fg(Color::Green)),
                Span::raw(": "),
                Span::styled(truncated_value, Style::default().fg(Color::Yellow)),
            ]);

            ListItem::new(line)
        }
        ChannelMessage::Progress { thread_id, current, total, message } => {
            let percentage = (*current as f64 / *total as f64 * 100.0) as u8;
            let thread_prefix = format!("[{:?}] ", thread_id);
            let percent_text = format!("{}% ", percentage);
            let prefix_len = thread_prefix.chars().count() + percent_text.chars().count();

            let message_max_width = max_width.saturating_sub(prefix_len);
            let truncated_message = truncate_text(message, message_max_width);

            let line = Line::from(vec![
                Span::styled(thread_prefix, Style::default().fg(Color::DarkGray)),
                Span::styled(percent_text, Style::default().fg(Color::Magenta)),
                Span::raw(truncated_message),
            ]);

            ListItem::new(line)
        }
        ChannelMessage::SolverStatus { thread_id, status } => {
            use crate::backend::SolverState;

            let status_text = match status {
                SolverState::Solving => "Solving",
                SolverState::Completed => "Completed",
                SolverState::TerminatedEarly => "Terminated",
            };

            let thread_prefix = format!("[{:?}] ", thread_id);
            let status_label = "STATUS";
            let prefix_len = thread_prefix.chars().count() + status_label.chars().count() + 1;

            let message_max_width = max_width.saturating_sub(prefix_len);
            let truncated_status = truncate_text(status_text, message_max_width);

            let line = Line::from(vec![
                Span::styled(thread_prefix, Style::default().fg(Color::DarkGray)),
                Span::styled(status_label, Style::default().fg(Color::Magenta)),
                Span::raw(" "),
                Span::styled(
                    truncated_status,
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ]);

            ListItem::new(line)
        }
    }
}

/// Formats a duration for display.
///
/// # Arguments
///
/// * `duration` - Duration to format
///
/// # Returns
///
/// A human-readable duration string (e.g., "1m 23s", "45.2s").
///
/// # Examples
///
/// ```
/// use solverforge_console::tui::widgets::format_duration;
/// use std::time::Duration;
///
/// let short = format_duration(Duration::from_millis(1234));
/// assert!(short.ends_with('s'));
///
/// let long = format_duration(Duration::from_secs(90));
/// assert!(long.contains('m'));
/// ```
pub fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();

    if secs >= 60 {
        let mins = secs / 60;
        let remaining_secs = secs % 60;
        format!("{}m {}s", mins, remaining_secs)
    } else if secs > 0 {
        format!("{}.{}s", secs, duration.subsec_millis() / 100)
    } else {
        format!("{}ms", duration.as_millis())
    }
}

/// Truncates text to fit within a maximum width, adding ellipsis if needed.
///
/// Uses Unicode character counting to ensure correct width calculation.
///
/// # Arguments
///
/// * `text` - Text to truncate
/// * `max_width` - Maximum width in characters
///
/// # Returns
///
/// Truncated text with "..." suffix if the original exceeds max_width.
///
/// # Examples
///
/// ```
/// use solverforge_console::tui::widgets::truncate_text;
///
/// let short = truncate_text("Hello", 10);
/// assert_eq!(short, "Hello");
///
/// let long = truncate_text("This is a very long message", 15);
/// assert_eq!(long, "This is a ve...");
/// assert_eq!(long.chars().count(), 15);
/// ```
pub fn truncate_text(text: &str, max_width: usize) -> String {
    let char_count = text.chars().count();

    if char_count <= max_width {
        text.to_string()
    } else if max_width <= 3 {
        "...".chars().take(max_width).collect()
    } else {
        let truncated: String = text.chars().take(max_width - 3).collect();
        format!("{}...", truncated)
    }
}
