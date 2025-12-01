use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use crate::app::{App, ConnectionStatus, StreamStatus};
use super::{plots, heatmap, motion, status};

pub fn render_main_layout(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Main layout: header, body, footer
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Body
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    // Render header
    render_header(frame, main_layout[0], app);

    // Body layout: left (plots + heatmap), right (status)
    let body_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Percentage(75),
            Constraint::Percentage(25),
        ])
        .split(main_layout[1]);

    // Left side: plots and heatmap
    let left_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Percentage(25),  // Amplitude plot
            Constraint::Percentage(40),  // Heatmap
            Constraint::Percentage(35),  // Motion timeline
        ])
        .split(body_layout[0]);

    plots::render_amplitude_plot(frame, left_layout[0], app.latest_frame.as_ref());
    heatmap::render_heatmap(frame, left_layout[1], &app.frame_history);
    motion::render_motion_timeline(
        frame,
        left_layout[2],
        &app.motion_timeline,
        app.motion_detector.config.detection_threshold,
        app.motion_result,
    );

    // Right side: status panel
    status::render_status_panel(frame, body_layout[1], app);

    // Render footer
    render_footer(frame, main_layout[2]);

    // Render help overlay if active
    if app.ui_state.show_help {
        render_help_overlay(frame, area);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let status_badges = vec![
        if app.connection_status == ConnectionStatus::Connected {
            Span::styled(" [Connected] ", Style::default().fg(Color::Green))
        } else {
            Span::styled(" [Disconnected] ", Style::default().fg(Color::Red))
        },
        if app.stream_status == StreamStatus::Streaming {
            Span::styled(" [Streaming] ", Style::default().fg(Color::Green))
        } else {
            Span::styled(" [Stopped] ", Style::default().fg(Color::Gray))
        },
        if app.motion_result.motion_detected {
            Span::styled(" [MOTION!] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(" [No Motion] ", Style::default().fg(Color::Green))
        },
    ];

    let header = Paragraph::new(Line::from(status_badges))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("ESP32-C3 CSI Motion Detector")
        )
        .alignment(Alignment::Right);

    frame.render_widget(header, area);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer_text = Line::from(vec![
        Span::styled("[q]", Style::default().fg(Color::Yellow)),
        Span::raw("Quit "),
        Span::styled("[c]", Style::default().fg(Color::Yellow)),
        Span::raw("Calibrate "),
        Span::styled("[↑↓]", Style::default().fg(Color::Yellow)),
        Span::raw("Threshold "),
        Span::styled("[←→]", Style::default().fg(Color::Yellow)),
        Span::raw("Smoothing "),
        Span::styled("[r]", Style::default().fg(Color::Yellow)),
        Span::raw("Reset "),
        Span::styled("[?]", Style::default().fg(Color::Yellow)),
        Span::raw("Help"),
    ]);

    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(Span::styled("Help", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  q/Esc   - Quit application"),
        Line::from("  ?       - Toggle this help"),
        Line::from(""),
        Line::from("Motion Detection:"),
        Line::from("  ↑/↓     - Increase/decrease threshold"),
        Line::from("  ←/→     - Decrease/increase smoothing"),
        Line::from("  c       - Recalibrate detector"),
        Line::from("  r       - Reset detector state"),
        Line::from(""),
        Line::from("Press any key to close..."),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title("Help")
        .title_alignment(Alignment::Center);

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .alignment(Alignment::Left);

    let popup_area = centered_rect(60, 60, area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
