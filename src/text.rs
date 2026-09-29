//! Small string helpers for rows that must fit a width: clipping by display
//! column, compact counts and durations, and capped name lists.

use std::path::Path;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::wrap;

/// Names [`listed`] shows before it counts the rest.
pub const LIST_MAX: usize = 8;

const MORE: &str = "… +";

/// `path` with the home directory folded to `~`.
pub fn short_path(path: &Path) -> String {
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
        && let Ok(rel) = path.strip_prefix(&home)
    {
        return format!("~/{}", rel.display());
    }
    path.display().to_string()
}

/// A count as a reader states it: `999`, `1.2k`, `3.4M`.
pub fn human_count(n: u64) -> String {
    match n {
        0..=999 => n.to_string(),
        1_000..=999_999 => trim_zero(n as f64 / 1_000.0, 'k'),
        _ => trim_zero(n as f64 / 1_000_000.0, 'M'),
    }
}

fn trim_zero(value: f64, unit: char) -> String {
    let text = format!("{value:.1}");
    let text = text.strip_suffix(".0").unwrap_or(&text);
    format!("{text}{unit}")
}

/// A running time: seconds under a minute, `3m 29s` past one.
pub fn elapsed(secs: u64) -> String {
    match secs >= 60 {
        true => format!("{}m {}s", secs / 60, secs % 60),
        false => format!("{secs}s"),
    }
}

/// `1 file`, `3 files`.
pub fn count_of(n: usize, noun: &str) -> String {
    format!("{n} {noun}{}", if n == 1 { "" } else { "s" })
}

/// Join names with commas, keeping the first [`LIST_MAX`] and counting the rest.
pub fn listed<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let all: Vec<&str> = names.collect();
    if all.len() <= LIST_MAX {
        return all.join(", ");
    }
    format!(
        "{} {MORE}{} more",
        all[..LIST_MAX].join(", "),
        all.len() - LIST_MAX
    )
}

/// Split a [`listed`] value into its names and the `… +N more` tail.
pub fn split_more(value: &str) -> (&str, Option<&str>) {
    match value.find(MORE) {
        Some(at) => (value[..at].trim_end(), Some(&value[at..])),
        None => (value, None),
    }
}

/// Clip to `max` display columns with an ellipsis. Measured in columns, not
/// chars, so wide CJK text still lands on one row.
pub fn clip_row(text: &str, max: usize) -> String {
    let flat = text.replace('\n', " ");
    if wrap::width(&flat) <= max {
        return flat;
    }
    let head = wrap::rows(&flat, max.saturating_sub(1).max(1))
        .first()
        .map(|r| flat[r.clone()].to_string())
        .unwrap_or_default();
    format!("{}…", head.trim_end())
}

/// `lines` as ANSI escapes, one row per line, for output that bypasses a
/// viewport: a plain `print!` still gets the theme's colours and weights.
pub fn to_ansi(lines: &[Line<'_>]) -> String {
    let mut out = String::new();
    for line in lines {
        for span in &line.spans {
            let style = line.style.patch(span.style);
            let mut codes = Vec::new();
            for (flag, code) in [
                (Modifier::BOLD, "1"),
                (Modifier::DIM, "2"),
                (Modifier::ITALIC, "3"),
                (Modifier::UNDERLINED, "4"),
                (Modifier::CROSSED_OUT, "9"),
            ] {
                if style.add_modifier.contains(flag) {
                    codes.push(code.to_string());
                }
            }
            codes.extend(style.fg.and_then(|c| sgr(c, 38)));
            codes.extend(style.bg.and_then(|c| sgr(c, 48)));
            match codes.is_empty() {
                true => out.push_str(&span.content),
                false => out.push_str(&format!("\x1b[{}m{}\x1b[0m", codes.join(";"), span.content)),
            }
        }
        out.push('\n');
    }
    out
}

/// One row of terminal output as spans, reading colour and weight escapes on
/// top of `base`. Other escapes (cursor moves, titles) are dropped.
pub fn from_ansi(row: &str, base: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut style = base;
    let mut run = String::new();
    let mut chars = row.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            run.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                let mut params = String::new();
                let mut end = None;
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        end = Some(c);
                        break;
                    }
                    params.push(c);
                }
                if end == Some('m') {
                    if !run.is_empty() {
                        spans.push(Span::styled(std::mem::take(&mut run), style));
                    }
                    style = apply_sgr(style, base, &params);
                }
            }
            Some(']') => {
                while let Some(c) = chars.next() {
                    if c == '\x07' || (c == '\x1b' && chars.peek() == Some(&'\\')) {
                        chars.next_if_eq(&'\\');
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    if !run.is_empty() {
        spans.push(Span::styled(run, style));
    }
    spans
}

const NAMED: [Color; 16] = [
    Color::Black,
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::Gray,
    Color::DarkGray,
    Color::LightRed,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightBlue,
    Color::LightMagenta,
    Color::LightCyan,
    Color::White,
];

fn apply_sgr(mut style: Style, base: Style, params: &str) -> Style {
    let codes: Vec<u16> = params.split(';').map(|p| p.parse().unwrap_or(0)).collect();
    let mut i = 0;
    while i < codes.len() {
        let code = codes[i];
        let extended = |at: usize| match codes.get(at + 1) {
            Some(5) => codes.get(at + 2).map(|&n| (Color::Indexed(n as u8), 3)),
            Some(2) => match (codes.get(at + 2), codes.get(at + 3), codes.get(at + 4)) {
                (Some(&r), Some(&g), Some(&b)) => Some((Color::Rgb(r as u8, g as u8, b as u8), 5)),
                _ => None,
            },
            _ => None,
        };
        let mut step = 1;
        style = match code {
            0 => base,
            1 => style.add_modifier(Modifier::BOLD),
            2 => style.add_modifier(Modifier::DIM),
            3 => style.add_modifier(Modifier::ITALIC),
            4 => style.add_modifier(Modifier::UNDERLINED),
            9 => style.add_modifier(Modifier::CROSSED_OUT),
            22 => style.remove_modifier(Modifier::BOLD | Modifier::DIM),
            23 => style.remove_modifier(Modifier::ITALIC),
            24 => style.remove_modifier(Modifier::UNDERLINED),
            29 => style.remove_modifier(Modifier::CROSSED_OUT),
            30..=37 => style.fg(NAMED[(code - 30) as usize]),
            90..=97 => style.fg(NAMED[(code - 90 + 8) as usize]),
            40..=47 => style.bg(NAMED[(code - 40) as usize]),
            100..=107 => style.bg(NAMED[(code - 100 + 8) as usize]),
            39 => Style {
                fg: base.fg,
                ..style
            },
            49 => Style {
                bg: base.bg,
                ..style
            },
            38 | 48 => match extended(i) {
                Some((color, used)) => {
                    step = used;
                    match code {
                        38 => style.fg(color),
                        _ => style.bg(color),
                    }
                }
                None => style,
            },
            _ => style,
        };
        i += step;
    }
    style
}

fn sgr(color: Color, base: u8) -> Option<String> {
    let named = |n: u8| Some(format!("{}", base - 8 + n));
    match color {
        Color::Reset => None,
        Color::Rgb(r, g, b) => Some(format!("{base};2;{r};{g};{b}")),
        Color::Indexed(i) => Some(format!("{base};5;{i}")),
        Color::Black => named(0),
        Color::Red => named(1),
        Color::Green => named(2),
        Color::Yellow => named(3),
        Color::Blue => named(4),
        Color::Magenta => named(5),
        Color::Cyan => named(6),
        Color::Gray => named(7),
        Color::DarkGray => Some(format!("{base};5;8")),
        Color::LightRed => Some(format!("{base};5;9")),
        Color::LightGreen => Some(format!("{base};5;10")),
        Color::LightYellow => Some(format!("{base};5;11")),
        Color::LightBlue => Some(format!("{base};5;12")),
        Color::LightMagenta => Some(format!("{base};5;13")),
        Color::LightCyan => Some(format!("{base};5;14")),
        Color::White => Some(format!("{base};5;15")),
    }
}

#[cfg(test)]
#[path = "tests/text_test.rs"]
mod tests;
