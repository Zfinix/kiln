//! Logos drawn in half blocks, so ten rows of pixels fit in five rows of text,
//! each pixel row tinted from the theme's `mark` gradient.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::theme;

/// A bitmap of lit columns per pixel row. At most ten rows, one per gradient stop.
pub type Bitmap = [&'static [usize]];

/// An eight-point star, the default mark.
pub const STAR: &Bitmap = &[
    &[4],
    &[1, 4, 7],
    &[2, 4, 6],
    &[3, 4, 5],
    &[0, 1, 2, 3, 4, 5, 6, 7, 8],
    &[3, 4, 5],
    &[2, 4, 6],
    &[1, 4, 7],
    &[4],
    &[],
];

/// `bitmap` as styled lines, `indent` columns in.
pub fn lines(bitmap: &Bitmap, indent: usize) -> Vec<Line<'static>> {
    let gradient = theme::get().mark;
    let columns = bitmap
        .iter()
        .flat_map(|row| row.iter())
        .max()
        .map_or(0, |c| c + 1);
    let lit = |row: usize, col: usize| bitmap.get(row).is_some_and(|r| r.contains(&col));
    let tint = |row: usize| gradient[row.min(gradient.len() - 1)];
    (0..bitmap.len())
        .step_by(2)
        .map(|top| {
            let bottom = top + 1;
            let mut spans = vec![Span::raw(" ".repeat(indent))];
            spans.extend(
                (0..columns).map(|col| match (lit(top, col), lit(bottom, col)) {
                    (true, true) => {
                        Span::styled("▀", Style::default().fg(tint(top)).bg(tint(bottom)))
                    }
                    (true, false) => Span::styled("▀", Style::default().fg(tint(top))),
                    (false, true) => Span::styled("▄", Style::default().fg(tint(bottom))),
                    (false, false) => Span::raw(" "),
                }),
            );
            Line::from(spans)
        })
        .collect()
}

/// The same lines as raw ANSI escapes, for output that bypasses ratatui.
pub fn ansi(lines: &[Line<'static>]) -> String {
    let mut out = String::new();
    for line in lines {
        for span in &line.spans {
            if let Some(Color::Rgb(r, g, b)) = span.style.fg {
                out.push_str(&format!("\x1b[38;2;{r};{g};{b}m"));
            }
            if let Some(Color::Rgb(r, g, b)) = span.style.bg {
                out.push_str(&format!("\x1b[48;2;{r};{g};{b}m"));
            }
            out.push_str(&span.content);
            out.push_str("\x1b[0m");
        }
        out.push('\n');
    }
    out
}
