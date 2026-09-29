//! Transcript cells. Every cell renders to a finished block of lines, already
//! wrapped, that the chat loop pushes into the terminal's own scrollback and never
//! touches again. That is what gives native scrolling, selection, and copy.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme;
use crate::wrap;

const GUTTER: usize = 2;
const HEAD: usize = 4;
const TAIL: usize = 4;

fn body_width(width: usize) -> usize {
    width.saturating_sub(GUTTER + 2).max(8)
}

fn hang(lines: Vec<Line<'static>>, bullet: Span<'static>, width: usize) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let mut first = true;
    for line in lines {
        for wrapped in wrap::wrap_line(line, body_width(width)) {
            let lead = if first {
                bullet.clone()
            } else {
                Span::raw(" ".repeat(GUTTER))
            };
            let mut spans = vec![lead];
            spans.extend(wrapped.spans);
            out.push(Line::from(spans).style(wrapped.style));
            first = false;
        }
    }
    out
}

fn bullet() -> Span<'static> {
    Span::styled("• ", theme::get().dimmer_style())
}

fn branch(first: bool) -> Span<'static> {
    if first {
        Span::styled("└ ", theme::get().faint_style())
    } else {
        Span::raw("  ")
    }
}

/// A message the user sent: a coral rail on a filled band, the only chapter
/// mark in the transcript.
pub fn user(text: &str, width: usize) -> Vec<Line<'static>> {
    let fill = Style::default()
        .fg(theme::get().text)
        .bg(theme::get().rail_bg);
    let body = body_width(width);
    let mut out = Vec::new();
    let mut first = true;
    for raw in text.lines() {
        for chunk in wrap::lines(raw, body) {
            let lead = if first { "❯ " } else { "  " };
            let line = Line::from(vec![
                Span::styled("▌", theme::get().accent_style().bg(theme::get().rail_bg)),
                Span::styled(lead, theme::get().accent_style().bg(theme::get().rail_bg)),
                Span::styled(chunk, fill),
            ]);
            out.push(wrap::pad_to(line, width.max(1), fill));
            first = false;
        }
    }
    prepend_blank(out)
}

/// Pre-rendered markdown, e.g. from [`crate::markdown`]. `first` draws the bullet; later
/// chunks of the same message continue under it.
pub fn assistant(lines: Vec<Line<'static>>, first: bool, width: usize) -> Vec<Line<'static>> {
    if lines.is_empty() {
        return Vec::new();
    }
    let lead = if first {
        bullet()
    } else {
        Span::raw(" ".repeat(GUTTER))
    };
    let block = hang(lines, lead, width);
    if first { prepend_blank(block) } else { block }
}

/// A short dim status line.
pub fn notice(text: &str, width: usize) -> Vec<Line<'static>> {
    hang(
        vec![Line::from(Span::styled(
            text.to_string(),
            theme::get().dimmer_style(),
        ))],
        Span::styled("· ", theme::get().dimmer_style()),
        width,
    )
}

pub fn error(text: &str, width: usize) -> Vec<Line<'static>> {
    let lines = text
        .lines()
        .map(|l| {
            Line::from(Span::styled(
                l.to_string(),
                Style::default().fg(theme::get().error),
            ))
        })
        .collect();
    prepend_blank(hang(
        lines,
        Span::styled("✗ ", Style::default().fg(theme::get().error)),
        width,
    ))
}

/// Failures framed in a red rounded box so they cannot be read as notes.
pub fn error_box(texts: &[String], width: usize) -> Vec<Line<'static>> {
    let style = theme::get().error_style();
    let inner = body_width(width).saturating_sub(4).max(8);
    let rows: Vec<String> = texts
        .iter()
        .flat_map(|text| wrap::lines(text, inner))
        .collect();
    let Some(box_width) = rows.iter().map(|row| wrap::width(row)).max() else {
        return Vec::new();
    };
    let margin = " ".repeat(GUTTER);
    let edge = "─".repeat(box_width + 2);
    let mut out = vec![Line::from(Span::styled(format!("{margin}╭{edge}╮"), style))];
    for row in rows {
        let pad = " ".repeat(box_width - wrap::width(&row));
        out.push(Line::from(Span::styled(
            format!("{margin}│ {row}{pad} │"),
            style,
        )));
    }
    out.push(Line::from(Span::styled(format!("{margin}╰{edge}╯"), style)));
    prepend_blank(out)
}

/// One row of a group that prints live. The first row opens the cell with
/// `header`; the rest hang off the same branch, so a run reads as one step.
pub fn group_row(header: &str, label: &str, open: bool, width: usize) -> Vec<Line<'static>> {
    let row = Line::from(vec![
        branch(!open),
        Span::styled(label.to_string(), Style::default().fg(theme::get().blue)),
    ]);
    if open {
        return hang(vec![row], Span::raw(" ".repeat(GUTTER)), width);
    }
    let header = Line::from(Span::styled(
        header.to_string(),
        Style::default()
            .fg(theme::get().text)
            .add_modifier(Modifier::BOLD),
    ));
    prepend_blank(hang(vec![header, row], bullet(), width))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Done,
    Skipped,
    Blocked,
}

/// A task list: a count line over one row per step. Done steps are struck
/// back to dim, the running one is accented, so the eye lands on what is live.
pub fn plan(steps: &[(StepStatus, String)], width: usize) -> Vec<Line<'static>> {
    if steps.is_empty() {
        return Vec::new();
    }
    let count = |want: StepStatus| steps.iter().filter(|(s, _)| *s == want).count();
    let mut parts = vec![format!("{} done", count(StepStatus::Done))];
    if count(StepStatus::InProgress) > 0 {
        parts.push(format!("{} in progress", count(StepStatus::InProgress)));
    }
    parts.push(format!("{} open", count(StepStatus::Pending)));
    for (status, label) in [
        (StepStatus::Blocked, "blocked"),
        (StepStatus::Skipped, "skipped"),
    ] {
        if count(status) > 0 {
            parts.push(format!("{} {label}", count(status)));
        }
    }

    let mut lines = vec![Line::from(vec![
        Span::styled(
            format!(
                "{} task{}",
                steps.len(),
                if steps.len() == 1 { "" } else { "s" }
            ),
            Style::default()
                .fg(theme::get().text)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" ({})", parts.join(", ")),
            theme::get().dimmer_style(),
        ),
    ])];

    for (status, label) in steps {
        let (glyph, style) = match status {
            StepStatus::Done => ("✔", theme::get().dimmer_style()),
            StepStatus::InProgress => ("◼", theme::get().accent_style()),
            StepStatus::Pending => ("◻", Style::default().fg(theme::get().dim)),
            StepStatus::Skipped => ("⊘", theme::get().faint_style()),
            StepStatus::Blocked => ("✖", Style::default().fg(theme::get().error)),
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{glyph} "), style),
            Span::styled(label.clone(), step_style(*status)),
        ]));
    }
    prepend_blank(hang(lines, bullet(), width))
}

fn step_style(status: StepStatus) -> Style {
    match status {
        StepStatus::Done => theme::get()
            .dimmer_style()
            .add_modifier(Modifier::CROSSED_OUT),
        StepStatus::Skipped => theme::get()
            .faint_style()
            .add_modifier(Modifier::CROSSED_OUT),
        StepStatus::InProgress => Style::default().fg(theme::get().text),
        StepStatus::Pending => Style::default().fg(theme::get().dim),
        StepStatus::Blocked => Style::default().fg(theme::get().error),
    }
}

/// A web search's sources: a bold header over one row per result, the title
/// in blue with the url dimmed beside it, capped by the caller.
pub fn sources(label: &str, sources: &[(String, String)], width: usize) -> Vec<Line<'static>> {
    let rows: Vec<Line<'static>> = sources
        .iter()
        .map(|(title, url)| {
            Line::from(vec![
                branch(false),
                Span::styled(title.clone(), Style::default().fg(theme::get().blue)),
                Span::raw(" "),
                Span::styled(url.clone(), theme::get().faint_style()),
            ])
        })
        .collect();
    let header = Line::from(Span::styled(
        label.to_string(),
        Style::default()
            .fg(theme::get().text)
            .add_modifier(Modifier::BOLD),
    ));
    let mut lines = vec![header];
    lines.extend(rows);
    prepend_blank(hang(lines, bullet(), width))
}

/// A tool call with its output, elided in the middle when it is long.
pub fn tool(label: &str, output: &str, failed: bool, width: usize) -> Vec<Line<'static>> {
    let head_style = if failed {
        Style::default()
            .fg(theme::get().error)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme::get().text)
            .add_modifier(Modifier::BOLD)
    };
    let mut lines = vec![Line::from(Span::styled(label.to_string(), head_style))];

    let body: Vec<&str> = output.lines().collect();
    let out_style = if failed {
        Style::default().fg(theme::get().error)
    } else {
        theme::get().dimmer_style()
    };
    for (i, text) in elide(&body).into_iter().enumerate() {
        let style = match text {
            Elided::Text(_) => out_style,
            Elided::Gap(_) => theme::get().faint_style(),
        };
        lines.push(Line::from(vec![
            branch(i == 0),
            Span::styled(text.into_string(), style),
        ]));
    }
    prepend_blank(hang(lines, bullet(), width))
}

/// Reasoning text. Collapsed it is one faint line naming its size, so a
/// long deliberation costs a row of scrollback; expanded it is the whole text,
/// dimmed to sit behind the answer rather than compete with it.
pub fn reasoning(text: &str, open: bool, width: usize) -> Vec<Line<'static>> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let header = Line::from(Span::styled(
        "Thinking".to_string(),
        Style::default()
            .fg(theme::get().text)
            .add_modifier(Modifier::BOLD),
    ));
    if !open {
        let words = text.split_whitespace().count();
        let hint = Line::from(vec![
            branch(true),
            Span::styled(format!("{words} words hidden"), theme::get().faint_style()),
        ]);
        return prepend_blank(hang(vec![header, hint], bullet(), width));
    }
    let mut lines = vec![header];
    for (i, body) in text.lines().enumerate() {
        lines.push(Line::from(vec![
            branch(i == 0),
            Span::styled(body.to_string(), theme::get().dimmer_style()),
        ]));
    }
    prepend_blank(hang(lines, bullet(), width))
}

/// An applied edit: `▸ verb path` with the counts pushed to the right edge,
/// then the tinted patch body.
pub fn patch(verb: &str, path: &str, body: &str, width: usize) -> Vec<Line<'static>> {
    let added = body.lines().filter(|l| l.starts_with('+')).count();
    let removed = body.lines().filter(|l| l.starts_with('-')).count();

    let inner = body_width(width);
    let left = format!("{verb} {path}");
    let right = format!("+{added} −{removed}");
    let gap = inner.saturating_sub(left.chars().count() + right.chars().count() + 1);
    let header = Line::from(vec![
        Span::styled(
            format!("{verb} "),
            Style::default()
                .fg(theme::get().text)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(path.to_string(), Style::default().fg(theme::get().blue)),
        Span::raw(" ".repeat(gap + 1)),
        Span::styled(
            format!("+{added}"),
            Style::default().fg(theme::get().add_fg),
        ),
        Span::raw(" "),
        Span::styled(
            format!("−{removed}"),
            Style::default().fg(theme::get().del_fg),
        ),
    ]);

    let mut lines = vec![header];
    lines.extend(diff_lines(body, inner));
    prepend_blank(hang(lines, bullet(), width))
}

/// Colour a unified-ish patch body, tinting the whole row so added and removed
/// lines read as bands. The `+`/`-` mark sits a step darker than its text.
pub fn diff_lines(body: &str, width: usize) -> Vec<Line<'static>> {
    body.lines()
        .map(|raw| {
            let (fg, bg, mark) = match raw.chars().next() {
                Some('+') => (
                    theme::get().add_fg,
                    theme::get().add_bg,
                    Some(theme::get().add_mark),
                ),
                Some('-') => (
                    theme::get().del_fg,
                    theme::get().del_bg,
                    Some(theme::get().del_mark),
                ),
                _ => (theme::get().faint, Color::Reset, None),
            };
            let style = Style::default().fg(fg).bg(bg);
            let text: String = wrap::lines(raw, width).first().cloned().unwrap_or_default();
            let line = match (mark, text.is_empty()) {
                (Some(mark_fg), false) => {
                    let (head, rest) = text.split_at(1);
                    Line::from(vec![
                        Span::styled(head.to_string(), Style::default().fg(mark_fg).bg(bg)),
                        Span::styled(rest.to_string(), style),
                    ])
                }
                _ => Line::from(Span::styled(text, style)),
            };
            wrap::pad_to(line, width, style)
        })
        .collect()
}

/// A session header, printed once above the first prompt: `mark`, the name and
/// version, the fields as aligned key/value rows, then an optional `hint` line.
pub fn header(
    mark: Vec<Line<'static>>,
    name: &str,
    version: &str,
    fields: &[(&str, String)],
    hint: Option<&str>,
    width: usize,
) -> Vec<Line<'static>> {
    let mut lines = mark;
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(name.to_string(), theme::get().accent_style()),
        Span::styled(format!("  {version}"), theme::get().dimmer_style()),
    ]));
    lines.push(Line::from(""));
    // Widest key plus a gap, so a key as long as the column still separates.
    let key_w = fields
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0)
        + 2;
    // Long values (tool or skill lists) wrap into a hanging indent under the
    // value column instead of running off the edge.
    let value_w = width.saturating_sub(key_w).max(16);
    for (key, value) in fields {
        let (names, more) = crate::text::split_more(value);
        let mut rows: Vec<Line<'static>> = wrap::lines(names, value_w)
            .into_iter()
            .map(|row| Line::from(Span::styled(row, Style::default().fg(theme::get().text))))
            .collect();
        // The count of what was cut trails the names, dimmed, and drops to its
        // own row when the last one has no space left.
        if let Some(more) = more {
            let tail = Span::styled(more.to_string(), theme::get().dimmer_style());
            let fits = rows
                .last()
                .is_some_and(|l| l.width() + 1 + wrap::width(more) <= value_w);
            match fits {
                true => {
                    let last = rows.last_mut().expect("fits implies a last row");
                    last.spans.push(Span::raw(" "));
                    last.spans.push(tail);
                }
                false => rows.push(Line::from(tail)),
            }
        }
        for (i, row) in rows.into_iter().enumerate() {
            let head = match i {
                0 => Span::styled(format!("{key:<key_w$}"), theme::get().dimmer_style()),
                _ => Span::raw(" ".repeat(key_w)),
            };
            let mut spans = vec![head];
            spans.extend(row.spans);
            lines.push(Line::from(spans));
        }
    }
    if let Some(hint) = hint {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            hint.to_string(),
            theme::get().dimmer_style(),
        )));
    }
    lines.push(Line::from(""));
    prepend_blank(lines)
}

/// A section header, e.g. `▶ Verify`.
pub fn phase(name: &str, width: usize) -> Vec<Line<'static>> {
    prepend_blank(hang(
        vec![Line::from(Span::styled(
            name.to_string(),
            theme::get().accent_style().add_modifier(Modifier::BOLD),
        ))],
        Span::styled("▶ ", theme::get().accent_style()),
        width,
    ))
}

fn prepend_blank(mut lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    if lines.is_empty() {
        return lines;
    }
    lines.insert(0, Line::from(""));
    lines
}

enum Elided<'a> {
    Text(&'a str),
    Gap(usize),
}

impl Elided<'_> {
    fn into_string(self) -> String {
        match self {
            Elided::Text(s) => s.to_string(),
            Elided::Gap(n) => format!("… +{n} lines"),
        }
    }
}

fn elide<'a>(body: &[&'a str]) -> Vec<Elided<'a>> {
    if body.len() <= HEAD + TAIL + 1 {
        return body.iter().copied().map(Elided::Text).collect();
    }
    let mut out: Vec<Elided<'a>> = body[..HEAD].iter().copied().map(Elided::Text).collect();
    out.push(Elided::Gap(body.len() - HEAD - TAIL));
    out.extend(body[body.len() - TAIL..].iter().copied().map(Elided::Text));
    out
}

#[cfg(test)]
#[path = "tests/cells_test.rs"]
mod tests;
