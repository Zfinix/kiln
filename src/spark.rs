//! One-row charts in block characters: a history of values at a glance.

use ratatui::style::Style;
use ratatui::text::Span;

const BARS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// The last `width` of `values` as bars scaled to the largest of them.
/// Shorter histories are padded on the left so the newest value sits right.
pub fn line(values: &[f64], width: usize, style: Style) -> Span<'static> {
    let shown = &values[values.len().saturating_sub(width)..];
    let peak = shown.iter().copied().fold(0.0_f64, f64::max);
    let bars: String = shown
        .iter()
        .map(|&v| match peak > 0.0 {
            true => BARS[((v.max(0.0) / peak) * 8.0).round() as usize],
            false => BARS[0],
        })
        .collect();
    Span::styled(format!("{bars:>width$}"), style)
}

#[cfg(test)]
#[path = "tests/spark_test.rs"]
mod tests;
