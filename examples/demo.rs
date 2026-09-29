//! A scripted tour of kiln: type a message and a canned reply streams in
//! through the markdown renderer, `/theme` opens a picker that previews as you
//! move, and every finished block lands in the terminal's own scrollback.

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Position;
use tokio::sync::mpsc;

use kiln::cells::{self, StepStatus};
use kiln::composer::Composer;
use kiln::guard::TuiGuard;
use kiln::list::{ListSelectionView, SelectionItem};
use kiln::markdown::MarkdownStream;
use kiln::menu::{Command, Menu};
use kiln::render::{Column, Inset, Insets, Renderable};
use kiln::status::StatusWidget;
use kiln::terminal::{Tui, TuiEvent, restore_raw};
use kiln::view::View;
use kiln::{mark, theme};

const REPLY: &str = "Here is what **kiln** gives you:

| Component | What it does |
|---|---|
| `InlineTerm` | a live pane under real scrollback |
| `Composer` | multi-line input with history |
| `MarkdownStream` | renders text as it streams |

```rust
let mut tui = Tui::new(3)?;
tui.insert_history(cells::notice(\"ready\", width))?;
```

Scroll up: everything above the prompt is plain terminal output.
";

#[derive(Clone)]
enum Event {
    Theme(String),
    Chunk(String),
    Done,
}

struct App {
    composer: Composer,
    menu: Menu,
    view: Option<Box<dyn View>>,
    status: Option<StatusWidget>,
    stream: MarkdownStream,
    first_chunk: bool,
    quit: bool,
}

impl App {
    fn pane(&self, width: u16) -> (Column<'_>, Option<(u16, u16)>) {
        let mut column = Column::new();
        if let Some(status) = &self.status {
            column.push(Inset::new(status, Insets::tlbr(1, 2, 0, 0)));
        }
        if let Some(view) = &self.view {
            column.push(Inset::new(view.as_ref(), Insets::tlbr(1, 2, 0, 2)));
            return (column, None);
        }
        let (lines, (row, col)) = self.composer.render(width, "Ask anything, / for commands");
        let menu = self.menu.lines(width.saturating_sub(2) as usize);
        let top = 1 + self.status.as_ref().map_or(0, |_| 2);
        column.push(Inset::new(lines, Insets::tlbr(1, 0, 0, 0)));
        column.push(Inset::new(menu, Insets::tlbr(0, 2, 0, 0)));
        (column, Some((row + top, col)))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = TuiGuard::install(restore_raw);
    if let Some(entry) = theme::named("ocean") {
        theme::set(entry.theme);
        theme::settle();
    }
    let mut tui = Tui::new(3)?;
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    let width = tui.width() as usize;
    let fields = [
        ("crate", "kiln".to_string()),
        ("theme", "ocean".to_string()),
        ("cwd", kiln::text::short_path(&std::env::current_dir()?)),
    ];
    tui.insert_history(cells::header(
        mark::lines(mark::STAR, 0),
        "kiln demo",
        env!("CARGO_PKG_VERSION"),
        &fields,
        Some("/ for commands  ·  esc quits"),
        width,
    ))?;

    let mut app = App {
        composer: Composer::default(),
        menu: Menu::default(),
        view: None,
        status: None,
        stream: MarkdownStream::default(),
        first_chunk: true,
        quit: false,
    };
    app.menu.set_commands(vec![
        Command::new("theme", "Switch the colour theme"),
        Command::new("plan", "Show a sample task list"),
        Command::new("diff", "Show a sample edit"),
        Command::new("quit", "Leave the demo"),
    ]);

    while !app.quit {
        let width = tui.width();
        let (pane, cursor) = app.pane(width);
        let height = pane.desired_height(width);
        tui.draw(height, |frame| {
            let area = frame.area();
            pane.render(area, frame.buffer_mut());
            if let Some((row, col)) = cursor {
                frame.set_cursor_position(Position::new(area.x + col, area.y + row));
            }
        })?;
        drop(pane);

        tokio::select! {
            event = tui.next_event() => match event {
                TuiEvent::Key(key) => on_key(&mut app, &mut tui, key, &tx)?,
                TuiEvent::Paste(text) => app.composer.paste(&text),
                TuiEvent::Resize => tui.resized()?,
                TuiEvent::Mouse(_) | TuiEvent::Draw => {}
            },
            Some(event) = rx.recv() => on_event(&mut app, &mut tui, event)?,
        }
        if theme::is_transitioning() {
            tui.frame_requester().schedule_in(Duration::from_millis(16));
        }
    }
    Ok(())
}

fn on_event(app: &mut App, tui: &mut Tui, event: Event) -> Result<()> {
    let width = tui.width() as usize;
    match event {
        Event::Theme(name) => {
            if let Some(entry) = theme::named(&name) {
                theme::set(entry.theme);
            }
            tui.insert_history(cells::notice(&format!("Theme set to {name}"), width))?;
        }
        Event::Chunk(delta) => {
            let lines = app.stream.push(&delta);
            if !lines.is_empty() {
                tui.insert_history(cells::assistant(lines, app.first_chunk, width))?;
                app.first_chunk = false;
            }
        }
        Event::Done => {
            let lines = app.stream.flush();
            tui.insert_history(cells::assistant(lines, app.first_chunk, width))?;
            app.status = None;
        }
    }
    Ok(())
}

fn on_key(
    app: &mut App,
    tui: &mut Tui,
    key: KeyEvent,
    tx: &mpsc::UnboundedSender<Event>,
) -> Result<()> {
    if let Some(view) = &mut app.view {
        view.handle_key(key);
        if view.is_complete() {
            app.view = None;
        }
        return Ok(());
    }
    let width = tui.width();
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Char('c') if ctrl => app.quit = true,
        KeyCode::Esc if app.menu.is_open() => app.menu.close(),
        KeyCode::Esc => app.quit = true,
        KeyCode::Up if app.menu.is_open() => app.menu.move_by(-1),
        KeyCode::Down if app.menu.is_open() => app.menu.move_by(1),
        KeyCode::Tab if app.menu.is_open() => {
            if let Some(command) = app.menu.picked() {
                let line = format!("/{}", command.name);
                app.composer.clear();
                app.composer.insert_str(&line);
            }
        }
        KeyCode::Enter => {
            let picked = app.menu.picked().map(|c| c.name.clone());
            app.menu.close();
            let text = app.composer.take();
            match picked.as_deref().or_else(|| text.strip_prefix('/')) {
                Some(command) => run_command(app, tui, command, tx)?,
                None if !text.trim().is_empty() => send(app, tui, &text, tx)?,
                None => {}
            }
        }
        KeyCode::Up => {
            if !app.composer.up(width) {
                app.composer.recall_prev();
            }
        }
        KeyCode::Down => {
            if !app.composer.down(width) {
                app.composer.recall_next();
            }
        }
        KeyCode::Left => app.composer.left(),
        KeyCode::Right => app.composer.right(),
        KeyCode::Home => app.composer.home(),
        KeyCode::End => app.composer.end(),
        KeyCode::Backspace => app.composer.backspace(),
        KeyCode::Delete => app.composer.delete(),
        KeyCode::Char('w') if ctrl => app.composer.delete_word_back(),
        KeyCode::Char('u') if ctrl => app.composer.kill_to_start(),
        KeyCode::Char('k') if ctrl => app.composer.kill_to_end(),
        KeyCode::Char(c) => app.composer.insert(c),
        _ => {}
    }
    app.menu.sync(app.composer.text());
    Ok(())
}

fn send(app: &mut App, tui: &mut Tui, text: &str, tx: &mpsc::UnboundedSender<Event>) -> Result<()> {
    let width = tui.width() as usize;
    tui.insert_history(cells::user(text, width))?;
    tui.insert_history(cells::group_row(
        "Explored",
        "Read src/lib.rs",
        false,
        width,
    ))?;
    tui.insert_history(cells::group_row(
        "Explored",
        "Searched \u{201c}pub mod\u{201d}",
        true,
        width,
    ))?;

    let mut status = StatusWidget::new(tui.frame_requester());
    status.set_detail(Some("Writing a reply".into()));
    app.status = Some(status);
    app.stream = MarkdownStream::default();
    app.stream.set_width(width.saturating_sub(4));
    app.first_chunk = true;

    let tx = tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(900)).await;
        for word in REPLY.split_inclusive(' ') {
            if tx.send(Event::Chunk(word.to_string())).is_err() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(18)).await;
        }
        let _ = tx.send(Event::Done);
    });
    Ok(())
}

fn run_command(
    app: &mut App,
    tui: &mut Tui,
    command: &str,
    tx: &mpsc::UnboundedSender<Event>,
) -> Result<()> {
    let width = tui.width() as usize;
    match command {
        "theme" => {
            let current = theme::get().accent;
            let items = theme::all()
                .iter()
                .map(|entry| SelectionItem {
                    name: entry.name.clone(),
                    description: entry.description.clone(),
                    is_current: entry.theme.accent == current,
                    event: Event::Theme(entry.name.clone()),
                })
                .collect();
            let view = ListSelectionView::new("Theme", items, tx.clone(), None).with_on_hover(
                Box::new(|item: &SelectionItem<Event>| {
                    if let Some(entry) = theme::named(&item.name) {
                        theme::set(entry.theme);
                    }
                }),
            );
            app.view = Some(Box::new(view));
        }
        "plan" => tui.insert_history(cells::plan(
            &[
                (StepStatus::Done, "Port the inline viewport".into()),
                (StepStatus::Done, "Port the composer".into()),
                (StepStatus::InProgress, "Record the demo".into()),
                (StepStatus::Pending, "Publish the crate".into()),
            ],
            width,
        ))?,
        "diff" => tui.insert_history(cells::patch(
            "Edited",
            "src/lib.rs",
            "@@ -1,2 +1,3 @@\n pub mod cells;\n-pub mod history;\n+pub mod menu;\n+pub mod approval;",
            width,
        ))?,
        "quit" => app.quit = true,
        other => tui.insert_history(cells::error_box(
            &[format!("There is no /{other} command. Type / to see the list.")],
            width,
        ))?,
    }
    Ok(())
}
