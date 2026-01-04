//! TUI layout management.
//!
//! Defines the multi-pane layout structure for the SERIO Console TUI.
//! The layout consists of:
//! - Header: Banner and job information
//! - Overview: Global metrics and solver state
//! - Thread Activity: Per-thread progress indicators
//! - Channels: Scrollable output per channel

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Panel areas for the TUI layout.
///
/// Contains the calculated rectangles for each UI panel.
///
/// # Examples
///
/// ```
/// use solverforge_console::tui::layout::calculate_layout;
/// use ratatui::layout::Rect;
///
/// // Terminal area: 80x24
/// let terminal_area = Rect::new(0, 0, 80, 24);
/// let panels = calculate_layout(terminal_area, 4); // 4 active threads
///
/// assert!(panels.header.height == 1);
/// assert!(panels.overview.height >= 3);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PanelAreas {
    /// Header area (banner and job info).
    pub header: Rect,
    /// Overview panel area (global metrics).
    pub overview: Rect,
    /// Thread activity panel area.
    pub thread_activity: Rect,
    /// Channels panel area (scrollable logs).
    pub channels: Rect,
}

/// Calculates the layout for all panels.
///
/// Divides the terminal area into header, overview, thread activity, and channel panels.
/// The layout is responsive to terminal size and number of active threads.
///
/// # Arguments
///
/// * `area` - Total terminal area available
/// * `thread_count` - Number of active solver threads (affects thread activity panel height)
///
/// # Examples
///
/// ```
/// use solverforge_console::tui::layout::calculate_layout;
/// use ratatui::layout::Rect;
///
/// // Small terminal: 80x24
/// let small_terminal = Rect::new(0, 0, 80, 24);
/// let layout = calculate_layout(small_terminal, 2);
///
/// // Layout should fit within terminal bounds
/// assert!(layout.header.y == 0);
/// assert!(layout.channels.y + layout.channels.height <= 24);
/// ```
pub fn calculate_layout(area: Rect, thread_count: usize) -> PanelAreas {
    // Main vertical split: [header][body]
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),      // Header
            Constraint::Min(0),         // Body
        ])
        .split(area);

    let header = main_chunks[0];
    let body = main_chunks[1];

    // Body split: [overview][thread_activity][channels]
    let overview_height = 3;
    let thread_activity_height = (thread_count as u16).max(1).saturating_add(2); // threads + border

    let body_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(overview_height),
            Constraint::Length(thread_activity_height),
            Constraint::Min(5), // Channels (at least 5 lines)
        ])
        .split(body);

    PanelAreas {
        header,
        overview: body_chunks[0],
        thread_activity: body_chunks[1],
        channels: body_chunks[2],
    }
}

/// Calculates the channel panel split for multiple channels.
///
/// Divides the channels area evenly among active channels.
///
/// # Arguments
///
/// * `channels_area` - Total area available for channel panels
/// * `channel_count` - Number of active channels to display
///
/// # Examples
///
/// ```
/// use solverforge_console::tui::layout::calculate_channel_split;
/// use ratatui::layout::Rect;
///
/// let channels_area = Rect::new(0, 10, 80, 14);
/// let splits = calculate_channel_split(channels_area, 2);
///
/// // Two channels should split the area
/// assert_eq!(splits.len(), 2);
/// assert!(splits[0].height > 0);
/// assert!(splits[1].height > 0);
/// ```
pub fn calculate_channel_split(channels_area: Rect, channel_count: usize) -> Vec<Rect> {
    if channel_count == 0 {
        return vec![channels_area];
    }

    let constraints: Vec<Constraint> = (0..channel_count)
        .map(|_| Constraint::Percentage(100 / channel_count as u16))
        .collect();

    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(channels_area)
        .to_vec()
}
