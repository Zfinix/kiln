//! A chat client for any Agent Client Protocol agent, `aster acp` by default,
//! built from kiln's pieces. Only the pane under the transcript is live;
//! finished output goes into the terminal's own scrollback.

mod acp;
mod keys;
mod labels;
mod model;
mod render;

use std::io;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::time::Duration;

use anyhow::{Result, bail};
use crossterm::cursor::MoveTo;
use crossterm::event::{self, EnableBracketedPaste, Event as TermEvent, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType, enable_raw_mode};
use ratatui::layout::Position;
use ratatui::text::Line;

use kiln::guard::TuiGuard;
use kiln::render::Renderable;
use kiln::term::InlineTerm;
use kiln::terminal::restore_raw;
use kiln::{cells, mark, text, theme};

use acp::{Client, Event};
use model::App;

const TICK: Duration = Duration::from_millis(90);

fn main() {
    if let Err(e) = run() {
        eprintln!("kiln chat: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut agent = std::env::var("KILN_AGENT").unwrap_or_else(|_| "aster".into());
    let mut theme_name = std::env::var("KILN_THEME").unwrap_or_else(|_| "ocean".into());
    let names = || {
        theme::all()
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>()
    };

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--agent" => agent = args.next().unwrap_or(agent),
            "--theme" => theme_name = args.next().unwrap_or(theme_name),
            "--themes" => {
                println!("{}", names().join("\n"));
                return Ok(());
            }
            "--version" => {
                println!("kiln chat {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            other => {
                bail!("there is no {other} option, try --agent, --theme, --themes or --version")
            }
        }
    }

    let Some(entry) = theme::named(&theme_name) else {
        bail!(
            "there is no theme called {theme_name}, try one of: {}",
            names().join(", ")
        );
    };
    theme::set(entry.theme);
    theme::settle();

    let cwd = std::env::current_dir()?;
    let (client, events) = Client::dial(&agent, &["acp"], &cwd.to_string_lossy())?;
    let client = Arc::new(client);

    // One worker owns the blocking prompt call, so the UI thread never stalls.
    let (turns, turns_rx) = channel::<String>();
    let (done_tx, done) = channel::<Option<String>>();
    let worker = client.clone();
    std::thread::spawn(move || {
        for message in turns_rx {
            let outcome = worker.prompt(&message).err().map(|e| e.to_string());
            if done_tx.send(outcome).is_err() {
                return;
            }
        }
    });

    let _guard = TuiGuard::install(restore_raw);
    enable_raw_mode()?;
    execute!(io::stdout(), EnableBracketedPaste)?;
    let mut term = InlineTerm::new(1)?;
    let mut app = App::new(client, theme_name.clone(), term.width());

    let fields = [
        ("agent", app.agent.clone()),
        ("cwd", text::short_path(&cwd)),
        ("mode", app.mode_name.to_lowercase()),
        ("theme", theme_name),
    ];
    term.insert_lines(cells::header(
        mark::lines(mark::STAR, 0),
        "kiln chat",
        env!("CARGO_PKG_VERSION"),
        &fields,
        Some("/ for commands    ctrl+c twice to quit"),
        term.width() as usize,
    ))?;

    // Finished lines wait here while an approval is open, so the question cannot
    // scroll away mid-decision.
    let mut held: Vec<Line<'static>> = Vec::new();

    while !app.should_quit {
        while let Ok(event) = events.try_recv() {
            held.extend(app.on_event(event));
        }
        if let Ok(error) = done.try_recv() {
            held.extend(app.on_event(Event::TurnEnded(error)));
        }

        if event::poll(TICK)? {
            match event::read()? {
                TermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                    held.extend(keys::on_key(&mut app, key, &turns));
                }
                TermEvent::Paste(pasted) if app.view.is_none() && !app.approvals.is_open() => {
                    app.composer.paste(&pasted);
                    app.menu.sync(app.composer.text());
                }
                TermEvent::Resize(..) => {
                    term.resized()?;
                    app.set_width(term.width());
                }
                _ => {}
            }
        } else if app.working {
            app.frame = app.frame.wrapping_add(1);
        }
        held.extend(app.take_picks());
        if !app.approvals.is_open() && !held.is_empty() {
            term.insert_lines(std::mem::take(&mut held))?;
        }

        let width = term.width();
        let (pane, cursor) = render::pane(&app, width);
        term.set_height(pane.desired_height(width))?;
        term.draw(|frame| {
            let area = frame.area();
            pane.render(area, frame.buffer_mut());
            if let Some((row, col)) = cursor {
                frame.set_cursor_position(Position::new(area.x + col, area.y + row));
            }
        })?;
    }

    execute!(
        io::stdout(),
        MoveTo(0, term.viewport_top()),
        Clear(ClearType::FromCursorDown)
    )?;
    Ok(())
}
