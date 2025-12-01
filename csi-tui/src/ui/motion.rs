use ratatui::{
    prelude::*,
    widgets::*,
};
use std::collections::VecDeque;
use std::time::SystemTime;
use crate::motion::MotionDetectionResult;

/// Render motion score timeline
pub fn render_motion_timeline(
    frame: &mut Frame,
    area: Rect,
    timeline: &VecDeque<(SystemTime, f32)>,
    threshold: f32,
    result: MotionDetectionResult,
) {
    if timeline.is_empty() {
        let placeholder = Paragraph::new("Waiting for motion data...")
            .block(Block::default().borders(Borders::ALL).title("Motion Score Timeline"))
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(placeholder, area);
        return;
    }

    // Convert to chart data
    let data: Vec<(f64, f64)> = timeline
        .iter()
        .enumerate()
        .map(|(i, (_, score))| (i as f64, *score as f64))
        .collect();

    // Threshold line
    let threshold_data: Vec<(f64, f64)> = vec![
        (0.0, threshold as f64),
        (data.len() as f64, threshold as f64),
    ];

    let datasets = vec![
        Dataset::default()
            .name("Motion Score")
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(if result.motion_detected {
                Color::Red
            } else {
                Color::Green
            }))
            .graph_type(GraphType::Line)
            .data(&data),
        Dataset::default()
            .name("Threshold")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Yellow))
            .graph_type(GraphType::Line)
            .data(&threshold_data),
    ];

    let title = if result.calibrating {
        "Motion Score Timeline [CALIBRATING...]"
    } else if result.motion_detected {
        "Motion Score Timeline [MOTION DETECTED!]"
    } else {
        "Motion Score Timeline [No Motion]"
    };

    let title_style = if result.motion_detected {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(title, title_style))
        )
        .x_axis(
            Axis::default()
                .title("Time (frames)")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, data.len() as f64])
        )
        .y_axis(
            Axis::default()
                .title("Score")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, 1.0])
                .labels(vec![Span::from("0.0"), Span::from("0.5"), Span::from("1.0")])
        );

    frame.render_widget(chart, area);
}
