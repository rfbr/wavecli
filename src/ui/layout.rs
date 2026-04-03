use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub file_browser: Rect,
    pub info: Rect,
    pub waveform: Rect,
    pub spectrogram: Rect,
    pub controls: Rect,
}

pub fn build_layout(
    area: Rect,
    show_waveform: bool,
    show_spectrogram: bool,
    show_file_browser: bool,
) -> AppLayout {
    // Split horizontally: file browser | main content
    let h_chunks = if show_file_browser {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(40)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(0), Constraint::Min(40)])
            .split(area)
    };

    let file_browser = h_chunks[0];
    let main_area = h_chunks[1];

    // Split main area vertically
    let mut constraints = vec![Constraint::Length(3)]; // info bar

    match (show_waveform, show_spectrogram) {
        (true, true) => {
            constraints.push(Constraint::Percentage(40));
            constraints.push(Constraint::Percentage(40));
        }
        (true, false) => {
            constraints.push(Constraint::Min(5));
            constraints.push(Constraint::Length(0));
        }
        (false, true) => {
            constraints.push(Constraint::Length(0));
            constraints.push(Constraint::Min(5));
        }
        (false, false) => {
            constraints.push(Constraint::Length(0));
            constraints.push(Constraint::Length(0));
        }
    }

    constraints.push(Constraint::Length(3)); // controls

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(main_area);

    AppLayout {
        file_browser,
        info: chunks[0],
        waveform: chunks[1],
        spectrogram: chunks[2],
        controls: chunks[3],
    }
}
