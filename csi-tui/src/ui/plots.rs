use ratatui::{
    prelude::*,
    widgets::*,
};
use crate::csi::frame::CsiFrame;

/// Render CSI amplitude plot
pub fn render_amplitude_plot(frame: &mut Frame, area: Rect, csi_frame: Option<&CsiFrame>) {
    let Some(csi) = csi_frame else {
        let placeholder = Paragraph::new("Waiting for CSI data...")
            .block(Block::default().borders(Borders::ALL).title("Live CSI Amplitude"))
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(placeholder, area);
        return;
    };

    // Convert amplitudes to chart data
    let data: Vec<(f64, f64)> = csi.amplitudes
        .iter()
        .enumerate()
        .map(|(i, &amp)| (i as f64, amp as f64))
        .collect();

    if data.is_empty() {
        return;
    }

    let max_amp = data.iter().map(|(_, y)| *y).fold(0.0, f64::max);
    let min_amp = data.iter().map(|(_, y)| *y).fold(f64::MAX, f64::min);

    let datasets = vec![
        Dataset::default()
            .name("Amplitude")
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(Color::Cyan))
            .graph_type(GraphType::Line)
            .data(&data)
    ];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Live CSI Amplitude")
        )
        .x_axis(
            Axis::default()
                .title("Subcarrier Index")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, data.len() as f64])
                .labels(vec![
                    Span::from("0"),
                    Span::from(format!("{}", data.len() / 4)),
                    Span::from(format!("{}", data.len() / 2)),
                    Span::from(format!("{}", 3 * data.len() / 4)),
                    Span::from(format!("{}", data.len())),
                ])
        )
        .y_axis(
            Axis::default()
                .title("Amplitude")
                .style(Style::default().fg(Color::Gray))
                .bounds([min_amp - 5.0, max_amp + 5.0])
                .labels(vec![
                    Span::from(format!("{:.0}", min_amp)),
                    Span::from(format!("{:.0}", (min_amp + max_amp) / 2.0)),
                    Span::from(format!("{:.0}", max_amp)),
                ])
        );

    frame.render_widget(chart, area);
}
