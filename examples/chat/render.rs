//! The live region only: an approval or picker, the slash menu, the composer and
//! one footer line. Everything above it already belongs to the terminal.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use kiln::render::{Column, Inset, Insets, Renderable};
use kiln::view::View;
use kiln::{SPINNER, text, theme};

use crate::model::App;

const PLACEHOLDER: &str = "Ask anything, / for commands";

struct ViewRef<'a>(&'a dyn View);

impl Renderable for ViewRef<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.0.render(area, buf);
    }
    fn desired_height(&self, width: u16) -> u16 {
        self.0.desired_height(width)
    }
}

/// The pane stacked top to bottom, and where the caret goes inside it, if the
/// composer has the keyboard. Height and drawing come from the same column.
pub fn pane(app: &App, width: u16) -> (Column<'_>, Option<(u16, u16)>) {
    let t = theme::get();
    let mut column = Column::new();

    if let Some(request) = app.approvals.current() {
        let mut head = vec![
            Span::styled("? ", t.accent_bold()),
            Span::styled(request.title.clone(), t.text_style()),
        ];
        if app.approvals.waiting() > 0 {
            head.push(Span::styled(
                format!("   +{} waiting", app.approvals.waiting()),
                t.dimmer_style(),
            ));
        }
        let mut lines = vec![Line::from(head)];
        let (preview, hidden) = app.approvals.preview();
        lines.extend(
            preview
                .into_iter()
                .map(|row| Line::styled(format!("  {row}"), t.dim_style())),
        );
        if hidden > 0 {
            lines.push(Line::styled(format!("  … {hidden} more"), t.dimmer_style()));
        }
        for (i, choice) in request.choices.iter().enumerate() {
            let (mark, style) = match (i == app.approvals.selected, choice.kind.allows()) {
                (true, true) => ("› ", t.accent_bold()),
                (true, false) => ("› ", t.error_style()),
                (false, _) => ("  ", t.dimmer_style()),
            };
            lines.push(Line::from(vec![
                Span::styled(mark, style),
                Span::styled(choice.label.clone(), style),
            ]));
        }
        lines.push(Line::styled(
            "↑↓ choose   enter confirm   esc deny",
            t.faint_style(),
        ));
        column.push(Inset::new(lines, Insets::tlbr(1, 2, 0, 0)));
    }

    if let Some(view) = &app.view {
        column.push(Inset::new(ViewRef(view.as_ref()), Insets::tlbr(1, 2, 0, 2)));
    }

    let (rows, hidden) = app.menu.window();
    let mut menu: Vec<Line<'static>> = rows
        .into_iter()
        .map(|(command, picked)| {
            let style = match picked {
                true => t.selected_style(),
                false => t.dim_style(),
            };
            let summary = command
                .description
                .split(['.', '\n'])
                .next()
                .unwrap_or_default();
            Line::from(vec![
                Span::styled(format!("/{}", command.name), style),
                Span::styled(
                    format!("  {}", text::clip_row(summary, 80)),
                    t.dimmer_style(),
                ),
            ])
        })
        .collect();
    if hidden > 0 {
        menu.push(Line::styled(format!("+{hidden} more"), t.faint_style()));
    }
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
