//! Permission prompts. Requests arrive while one is already open, queue behind
//! it, and each answer is logged to scrollback. Enter confirms, esc denies, and
//! the safe one-time allow is always preselected.

use std::time::Duration;

use anyhow::Result;
use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use kiln::approval::{Approvals, Choice, ChoiceKind, Request};
use kiln::cells;
use kiln::guard::TuiGuard;
use kiln::render::{Inset, Insets, Renderable};
use kiln::terminal::{Tui, TuiEvent, restore_raw};
use kiln::theme;

fn request(title: &str, preview: &[&str]) -> Request<()> {
    let choice = |id: &str, label: &str, kind| Choice {
        id: id.into(),
        label: label.into(),
        kind,
    };
    Request {
        title: title.into(),
        preview: preview.iter().map(|row| (*row).to_string()).collect(),
        choices: vec![
            choice("once", "Yes, this time", ChoiceKind::AllowOnce),
            choice(
                "always",
                "Yes, and don't ask again",
                ChoiceKind::AllowAlways,
            ),
            choice(
                "no",
                "No, tell it what to do instead",
                ChoiceKind::RejectOnce,
            ),
        ],
        payload: (),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Some(entry) = theme::named("nord") {
        theme::set(entry.theme);
        theme::settle();
    }
    let (tx, mut rx) = mpsc::unbounded_channel::<Request<()>>();
    tokio::spawn(async move {
        let incoming = [
            request("Run cargo publish --dry-run", &["cargo publish --dry-run"]),
            request(
                "Edit src/menu.rs",
                &[
                    "@@ -86,3 +86,3 @@",
                    "-        let style = if selected {",
                    "+        let style = match selected {",
                ],
            ),
            request("Fetch https://docs.rs/ratatui", &[]),
        ];
        for next in incoming {
            if tx.send(next).is_err() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(700)).await;
        }
    });

    let _guard = TuiGuard::install(restore_raw);
    let mut tui = Tui::new(8)?;
    let mut approvals = Approvals::default();
    let mut answered = 0;

    while answered < 3 {
        let pane = Inset::new(approvals.lines(), Insets::tlbr(1, 2, 0, 0));
        tui.draw(pane.desired_height(tui.width()), |frame| {
            pane.render(frame.area(), frame.buffer_mut());
        })?;
        tokio::select! {
            event = tui.next_event() => match event {
                TuiEvent::Key(key) => {
                    let outcome = match key.code {
                        KeyCode::Up => {
                            approvals.move_by(-1);
                            None
                        }
                        KeyCode::Down => {
                            approvals.move_by(1);
                            None
                        }
                        KeyCode::Enter => approvals.answer(),
                        KeyCode::Esc => approvals.reject(),
                        _ => None,
                    };
                    if let Some((request, choice)) = outcome {
                        answered += 1;
                        let width = tui.width() as usize;
                        let line = format!("{}: {}", choice.label, request.title);
                        tui.insert_history(match choice.kind.allows() {
                            true => cells::notice(&line, width),
                            false => cells::error(&line, width),
                        })?;
                    }
                }
                TuiEvent::Resize => tui.resized()?,
                TuiEvent::Mouse(_) | TuiEvent::Paste(_) | TuiEvent::Draw => {}
            },
            Some(next) = rx.recv() => approvals.push(next),
        }
    }
    Ok(())
}
