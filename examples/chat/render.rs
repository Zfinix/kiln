//! The live region only: an approval or picker, the slash menu, the composer and
//! one footer line. Everything above it already belongs to the terminal.

use ratatui::text::{Line, Span};

use kiln::render::{Column, Inset, Insets, Renderable};
use kiln::{SPINNER, theme};

use crate::model::App;

const PLACEHOLDER: &str = "Ask anything, / for commands";

/// The pane stacked top to bottom, and where the caret goes inside it, if the
/// composer has the keyboard. Height and drawing come from the same column.
pub fn pane(app: &App, width: u16) -> (Column<'_>, Option<(u16, u16)>) {
    let t = theme::get();
    let mut column = Column::new();

    if app.approvals.is_open() {
        column.push(Inset::new(app.approvals.lines(), Insets::tlbr(1, 2, 0, 0)));
    }

    if let Some(view) = &app.view {
        column.push(Inset::new(view.as_ref(), Insets::tlbr(1, 2, 0, 2)));
    }

    let menu = app.menu.lines(width.saturating_sub(2) as usize);
    column.push(Inset::new(menu, Insets::tlbr(0, 2, 0, 0)));

    let (lines, (row, col)) = app.composer.render(width, PLACEHOLDER);
    let top = column.desired_height(width) + 1;
    column.push(Inset::new(lines, Insets::tlbr(1, 0, 0, 0)));

    let mode = app.mode_name.to_lowercase();
    let glyph = match mode.as_str() {
        "plan" => "⏸",
        "manual" => "⏵",
        "edit" => "⏵⏵⏵",
        "yolo" => "☠",
        _ => "⏵⏵",
    };
    let mut footer = vec![
        Span::styled(format!("{glyph} {mode}"), t.accent_style()),
        Span::styled(format!("  ·  {}", app.agent), t.dimmer_style()),
    ];
    if app.working {
        let spinner = SPINNER[app.frame % SPINNER.len()];
        let doing = if app.thinking { "thinking" } else { "working" };
        footer.push(Span::styled(format!("  {spinner}"), t.accent_style()));
        footer.push(Span::styled(format!(" {doing}"), t.dimmer_style()));
    }
    if let Some(note) = app.flash.as_ref().or(app.title.as_ref()) {
        footer.push(Span::styled(format!("  ·  {note}"), t.dimmer_style()));
    }
    column.push(Inset::new(Line::from(footer), Insets::tlbr(0, 2, 0, 0)));

    let typing = app.view.is_none() && !app.approvals.is_open();
    (column, typing.then_some((top + row, col)))
}
