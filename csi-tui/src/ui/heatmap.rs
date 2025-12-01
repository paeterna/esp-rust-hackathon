use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, canvas::Canvas},
};
use std::collections::VecDeque;
use std::time::SystemTime;
use crate::csi::frame::CsiFrame;

/// Render CSI heatmap (time × subcarrier)
pub fn render_heatmap(
    frame: &mut Frame,
    area: Rect,
    history: &VecDeque<(SystemTime, CsiFrame)>,
) {
    if history.is_empty() {
        let placeholder = Paragraph::new("Waiting for CSI history...")
            .block(Block::default().borders(Borders::ALL).title("CSI Heatmap (Time × Subcarrier)"))
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(placeholder, area);
        return;
    }

    // Determine subcarrier count
    let num_subcarriers = history.back().map(|(_, f)| f.amplitudes.len()).unwrap_or(64);

    // Create canvas for heatmap
    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("CSI Heatmap (Time × Subcarrier)")
        )
        .x_bounds([0.0, history.len() as f64])
        .y_bounds([0.0, num_subcarriers as f64])
        .paint(|ctx| {
            for (time_idx, (_, csi)) in history.iter().enumerate() {
                // Normalize amplitudes for color mapping
                let max_amp = csi.amplitudes.iter().cloned().fold(0.0f32, f32::max);

                for (subcarrier_idx, &amplitude) in csi.amplitudes.iter().enumerate() {
                    let normalized = if max_amp > 0.0 {
                        amplitude / max_amp
                    } else {
                        0.0
                    };

                    // Map to color (blue -> green -> yellow -> red)
                    let color = amplitude_to_color(normalized);

                    // Draw pixel
                    ctx.print(
                        time_idx as f64,
                        subcarrier_idx as f64,
                        Span::styled("█", Style::default().fg(color))
                    );
                }
            }
        });

    frame.render_widget(canvas, area);
}

/// Map normalized amplitude (0.0-1.0) to color
fn amplitude_to_color(normalized: f32) -> Color {
    match normalized {
        x if x < 0.25 => Color::Blue,
        x if x < 0.5 => Color::Cyan,
        x if x < 0.75 => Color::Yellow,
        _ => Color::Red,
    }
}
