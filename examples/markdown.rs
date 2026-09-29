//! Streams a markdown file into scrollback a few words at a time, the way a
//! model's reply arrives, with a spinner in the pane until it is done. No async
//! runtime: this drives `InlineTerm` directly.
//!
//! `cargo run --example markdown -- README.md`

use std::io::stdout;
use std::time::Duration;

use anyhow::Result;
use crossterm::cursor::MoveTo;
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};

use kiln::cells;
use kiln::markdown::MarkdownStream;
use kiln::term::InlineTerm;
use kiln::{SPINNER, theme};

const SAMPLE: &str = "# Release notes

This release adds five examples and a few helpers:

- `Approvals::lines` draws a permission prompt
- `Menu::lines` draws the slash menu
- `text::to_ansi` prints cells *without* a viewport

| Example | Shows |
|---|---|
| pick | a filterable one-question picker |
| tasks | a live plan with a spinner |
| approve | queued permission prompts |

> Everything above the pane is ordinary terminal output.

```rust
let lines = kiln::markdown::render(\"**hello**\");
print!(\"{}\", kiln::text::to_ansi(&lines));
```
";

fn main() -> Result<()> {
    let source = match std::env::args().nth(1) {
        Some(path) => std::fs::read_to_string(path)?,
        None => SAMPLE.to_string(),
    };
    if let Some(entry) = theme::named("catppuccin") {
        theme::set(entry.theme);
        theme::settle();
    }

    let mut term = InlineTerm::new(1)?;
    let width = term.width() as usize;
    let mut stream = MarkdownStream::default();
    stream.set_width(width.saturating_sub(4));
    let mut first = true;

    for (frame, word) in source.split_inclusive(' ').enumerate() {
        let lines = stream.push(word);
        if !lines.is_empty() {
            term.insert_lines(cells::assistant(lines, first, width))?;
            first = false;
        }
        let spinner = Line::styled(
            format!("{} Streaming", SPINNER[frame / 3 % SPINNER.len()]),
            theme::get().dim_style(),
        );
        term.draw(|f| {
            let area = f.area();
            Paragraph::new(spinner).render(area, f.buffer_mut());
        })?;
        std::thread::sleep(Duration::from_millis(12));
    }
    term.insert_lines(cells::assistant(stream.flush(), first, width))?;
    execute!(
        stdout(),
        MoveTo(0, term.viewport_top()),
        Clear(ClearType::FromCursorDown)
    )?;
    Ok(())
}
