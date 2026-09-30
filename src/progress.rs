//! A progress bar filled with the theme's gradient, in eighths of a cell.

use ratatui::style::{Color, Style};
use ratatui::text::Span;

use crate::theme;

const PARTIAL: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];

/// `ratio` (clamped to 0..=1) of `width` cells filled, left to right from the
/// start of the theme's `mark` gradient to its end, the rest faint.
pub fn bar(ratio: f64, width: usize) -> Vec<Span<'static>> {
    let t = theme::get();
    let eighths = (ratio.clamp(0.0, 1.0) * width as f64 * 8.0).round() as usize;
    let (full, part) = (eighths / 8, eighths % 8);
    let tint = |cell: usize| {
        let at = cell as f32 / width.saturating_sub(1).max(1) as f32;
        blend(t.mark[0], t.mark[t.mark.len() - 1], at)
    };

    let mut spans: Vec<Span<'static>> = (0..full)
        .map(|cell| Span::styled("█", Style::default().fg(tint(cell))))
        .collect();
    if part > 0 {
        spans.push(Span::styled(
            PARTIAL[part].to_string(),
            Style::default().fg(tint(full)),
        ));
    }
    let used = full + usize::from(part > 0);
    if used < width {
        spans.push(Span::styled("─".repeat(width - used), t.faint_style()));
    }
    spans
}

fn blend(from: Color, to: Color, at: f32) -> Color {
    let (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) = (from, to) else {
        return if at < 0.5 { from } else { to };
    };
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * at).round() as u8;
    Color::Rgb(mix(ar, br), mix(ag, bg), mix(ab, bb))
}

#[cfg(test)]
#[path = "tests/progress_test.rs"]
mod tests;
