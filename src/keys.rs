//! Key hints: a one-row footer, and the fuller help list behind `?`.

use ratatui::text::{Line, Span};

use crate::theme;

/// One binding: the keys as the user types them, and what they do.
pub type Binding = (&'static str, &'static str);

/// `q quit · ? help` in one dim row.
pub fn footer(bindings: &[Binding]) -> Line<'static> {
    let t = theme::get();
    let mut spans = Vec::new();
    for (i, (keys, action)) in bindings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" · ", t.faint_style()));
        }
        spans.push(Span::styled(*keys, t.dim_style()));
        spans.push(Span::styled(format!(" {action}"), t.faint_style()));
    }
    Line::from(spans)
}

/// One row per binding, keys in an aligned column.
pub fn help(bindings: &[Binding]) -> Vec<Line<'static>> {
    let t = theme::get();
    let keys_w = bindings
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);
    bindings
        .iter()
        .map(|(keys, action)| {
            Line::from(vec![
                Span::styled(format!("{keys:>keys_w$}"), t.accent_style()),
                Span::styled(format!("  {action}"), t.dim_style()),
            ])
        })
        .collect()
}

#[cfg(test)]
#[path = "tests/keys_test.rs"]
mod tests;
