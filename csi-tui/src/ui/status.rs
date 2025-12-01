use ratatui::{
    prelude::*,
    widgets::*,
};
use crate::app::{App, ConnectionStatus, StreamStatus, ConfigParam};

/// Render status panel
pub fn render_status_panel(frame: &mut Frame, area: Rect, app: &App) {
    let connection_text = match app.connection_status {
        ConnectionStatus::Disconnected => Span::styled("Disconnected", Style::default().fg(Color::Red)),
        ConnectionStatus::Connecting => Span::styled("Connecting...", Style::default().fg(Color::Yellow)),
        ConnectionStatus::Connected => Span::styled("Connected", Style::default().fg(Color::Green)),
        ConnectionStatus::Error => Span::styled("Error", Style::default().fg(Color::Red)),
    };

    let stream_text = match app.stream_status {
        StreamStatus::Stopped => Span::styled("Stopped", Style::default().fg(Color::Gray)),
        StreamStatus::Starting => Span::styled("Starting...", Style::default().fg(Color::Yellow)),
        StreamStatus::Streaming => Span::styled("Streaming", Style::default().fg(Color::Green)),
        StreamStatus::Paused => Span::styled("Paused", Style::default().fg(Color::Yellow)),
    };

    let metadata_text = if let Some(frame) = &app.latest_frame {
        format!(
            "RSSI: {} dBm | Rate: {} Mbps | Ch: {} | BW: {} MHz",
            frame.metadata.rssi,
            frame.metadata.rate,
            frame.metadata.channel,
            frame.metadata.bandwidth
        )
    } else {
        "No data yet".to_string()
    };

    let config = &app.motion_detector.config;

    let lines = vec![
        Line::from(vec![
            Span::raw("Device: "),
            Span::styled(&app.device_path, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::raw("Status: "),
            connection_text,
            Span::raw(" | "),
            stream_text,
        ]),
        Line::from(Span::raw("")),
        Line::from(Span::styled("-- Signal Info --", Style::default().fg(Color::Gray))),
        Line::from(Span::raw(metadata_text)),
        Line::from(Span::raw("")),
        Line::from(Span::styled("-- Motion Config --", Style::default().fg(Color::Gray))),
        Line::from(vec![
            Span::raw("Threshold: "),
            Span::styled(
                format!("{:.2}", config.detection_threshold),
                if app.ui_state.selected_config_param == ConfigParam::Threshold {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                }
            ),
            Span::raw(" [↑↓]"),
        ]),
        Line::from(vec![
            Span::raw("Smoothing: "),
            Span::styled(
                format!("{:.2}", config.smoothing_alpha),
                if app.ui_state.selected_config_param == ConfigParam::Smoothing {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                }
            ),
            Span::raw(" [←→]"),
        ]),
        Line::from(format!("Window: {} frames", config.window_size)),
        Line::from(format!("Min Consecutive: {}", config.min_consecutive_frames)),
    ];

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Status & Config"))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
