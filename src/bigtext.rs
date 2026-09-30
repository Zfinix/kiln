//! Large digits in block characters, three rows tall, for timers and readouts.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// Rows every glyph is drawn in.
pub const HEIGHT: usize = 3;

fn glyph(c: char) -> [&'static str; HEIGHT] {
    match c {
        '0' => ["█▀█", "█ █", "█▄█"],
        '1' => ["▀█ ", " █ ", "▄█▄"],
        '2' => ["▀▀█", "█▀▀", "█▄▄"],
        '3' => ["▀▀█", " ▀█", "▄▄█"],
        '4' => ["█ █", "▀▀█", "  █"],
        '5' => ["█▀▀", "▀▀█", "▄▄█"],
        '6' => ["█▀▀", "█▀█", "█▄█"],
        '7' => ["▀▀█", "  █", "  █"],
        '8' => ["█▀█", "█▀█", "█▄█"],
        '9' => ["█▀█", "▀▀█", "▄▄█"],
        ':' => [" ", "▀", "▀"],
        '.' => [" ", " ", "▄"],
        '-' => ["   ", "▀▀▀", "   "],
        '%' => ["▀ █", " █ ", "█ ▄"],
        _ => ["  ", "  ", "  "],
    }
}

/// `text` as [`HEIGHT`] rows of block digits in `style`, one column apart.
/// Digits, `:`, `.`, `-` and `%` are drawn; anything else becomes a gap.
pub fn lines(text: &str, style: Style) -> Vec<Line<'static>> {
    (0..HEIGHT)
        .map(|row| {
            let cells: Vec<&str> = text.chars().map(|c| glyph(c)[row]).collect();
            Line::from(Span::styled(cells.join(" "), style))
        })
        .collect()
}

/// Columns [`lines`] takes for `text`.
pub fn width(text: &str) -> usize {
    let glyphs: usize = text.chars().map(|c| glyph(c)[0].chars().count()).sum();
    glyphs + text.chars().count().saturating_sub(1)
}

#[cfg(test)]
#[path = "tests/bigtext_test.rs"]
mod tests;
