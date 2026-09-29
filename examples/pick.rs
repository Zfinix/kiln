//! A one-question picker, in the spirit of `gum choose`: pass the options as
//! arguments, type to filter, press a number or enter to pick. The answer is
//! printed once the pane is gone; esc exits with status 1.

use anyhow::Result;
use tokio::sync::mpsc;

use kiln::guard::TuiGuard;
use kiln::list::{ListSelectionView, SelectionItem};
use kiln::render::{Inset, Insets, Renderable};
use kiln::terminal::{Tui, TuiEvent, restore_raw};
use kiln::theme;
use kiln::view::View;

const FALLBACK: [(&str, &str); 6] = [
    ("ratatui", "the widget and layout engine"),
    ("crossterm", "raw mode, events and escapes"),
    ("tokio", "the async runtime under Tui"),
    ("pulldown-cmark", "inline markdown parsing"),
    ("unicode-width", "column widths for CJK and emoji"),
    ("anyhow", "errors that carry context"),
];

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let title = match args.is_empty() {
        true => "Which crate does kiln lean on most?",
        false => "Pick one",
    };
    let items: Vec<(String, String)> = match args.is_empty() {
        true => FALLBACK
            .iter()
            .map(|(name, about)| (name.to_string(), about.to_string()))
            .collect(),
        false => args.into_iter().map(|name| (name, String::new())).collect(),
    };
    if let Some(entry) = theme::named("orchid") {
        theme::set(entry.theme);
        theme::settle();
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<Option<String>>();
    let mut view = ListSelectionView::new(
        title,
        items
            .into_iter()
            .map(|(name, description)| SelectionItem {
                event: Some(name.clone()),
                name,
                description,
                is_current: false,
            })
            .collect(),
        tx,
        Some(None),
    );

    let picked = {
        let _guard = TuiGuard::install(restore_raw);
        let mut tui = Tui::new(8)?;
        while !view.is_complete() {
            let pane = Inset::new(&view, Insets::tlbr(1, 2, 0, 2));
            tui.draw(pane.desired_height(tui.width()), |frame| {
                pane.render(frame.area(), frame.buffer_mut());
            })?;
            match tui.next_event().await {
                TuiEvent::Key(key) => view.handle_key(key),
                TuiEvent::Resize => tui.resized()?,
                TuiEvent::Mouse(_) | TuiEvent::Paste(_) | TuiEvent::Draw => {}
            }
        }
        rx.try_recv().ok().flatten()
    };

    match picked {
        Some(name) => println!("{name}"),
        None => std::process::exit(1),
    }
    Ok(())
}
