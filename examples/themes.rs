//! Every built-in theme, two to a row, printed as plain text with `text::to_ansi`: no raw
//! mode and no viewport, which is all a one-shot CLI needs.

use ratatui::text::{Line, Span};

use kiln::{mark, text, theme, wrap};

const COLUMN: usize = 62;

fn main() {
    let mut blocks = Vec::new();
    for entry in theme::all() {
        theme::set(entry.theme);
        theme::settle();
        let t = theme::get();
        let logo = mark::lines(mark::STAR, 2);
        let swatch = |color| Span::styled("██", ratatui::style::Style::default().fg(color));
        let mut rows = vec![
            Line::from(vec![
                Span::styled(format!("{:<12}", entry.name), t.accent_bold()),
                Span::styled(entry.description.clone(), t.dim_style()),
            ]),
            Line::from(
                [
                    t.accent, t.text, t.dim, t.blue, t.purple, t.amber, t.add_fg, t.del_fg,
                ]
                .map(swatch)
                .to_vec(),
            ),
            Line::from(vec![
                Span::styled("+ added ", t.add_row_style()),
                Span::raw(" "),
                Span::styled("- removed ", t.del_row_style()),
                Span::raw(" "),
                Span::styled(" code ", t.code_style()),
            ]),
        ];
        rows.resize(logo.len().max(rows.len()), Line::default());
        let mut block: Vec<Line<'static>> = logo
            .into_iter()
            .zip(rows)
            .map(|(art, row)| {
                let mut spans = art.spans;
                spans.push(Span::raw("   "));
                spans.extend(row.spans);
                Line::from(spans)
            })
            .collect();
        block.push(Line::default());
        blocks.push(block);
    }

    let mut out = Vec::new();
    for pair in blocks.chunks(2) {
        for (i, left) in pair[0].iter().enumerate() {
            let mut line = wrap::pad_to(left.clone(), COLUMN, Default::default());
            if let Some(right) = pair.get(1).and_then(|block| block.get(i)) {
                line.spans.extend(right.spans.clone());
            }
            out.push(line);
        }
    }
    print!("{}", text::to_ansi(&out));
}
