//! Install output in the style of `uv`: a spinner with a count and a thin bar
//! per download while work runs, then `Installed 3 packages in 174ms` and
//! ` + name==version` lines. Printed straight to stderr, no viewport needed.

use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use ratatui::crossterm::{cursor, queue, terminal};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::{SPINNER, text, theme, wrap};

/// Columns a download bar takes when the terminal has room for it.
pub const BAR_WIDTH: usize = 30;
/// Downloads drawn at once; the rest wait their turn below the fold.
pub const MAX_BARS: usize = 8;

const TICK: Duration = Duration::from_millis(80);
const LABEL_MAX: usize = 24;

/// What happened to one installed item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Added,
    Removed,
    Reinstalled,
}

/// `Installed 3 packages in 174ms`: the verb and time dim, the count bold.
pub fn summary(verb: &str, what: &str, took: Duration) -> Line<'static> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    Line::from(vec![
        Span::styled(format!("{verb} "), dim),
        Span::styled(
            what.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" in {}", duration(took)), dim),
    ])
}

/// ` + name==version`, with `-` for a removal and `~` for a reinstall.
pub fn change(kind: Change, name: &str, version: &str) -> Line<'static> {
    let t = theme::get();
    let (mark, color) = match kind {
        Change::Added => ("+", t.success),
        Change::Removed => ("-", t.error),
        Change::Reinstalled => ("~", t.warning),
    };
    Line::from(vec![
        " ".into(),
        Span::styled(
            mark,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        " ".into(),
        Span::styled(
            name.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("=={version}"),
            Style::default().add_modifier(Modifier::DIM),
        ),
    ])
}

/// `warning: message`, the label bold in the theme's warning colour.
pub fn warning(message: &str) -> Line<'static> {
    let label = Style::default()
        .fg(theme::get().warning)
        .add_modifier(Modifier::BOLD);
    Line::from(vec![
        Span::styled("warning:", label),
        format!(" {message}").into(),
    ])
}

/// Print `line` to stderr: styled on a terminal, plain text when piped.
pub fn emit(line: &Line<'_>) {
    let mut err = io::stderr().lock();
    let _ = match io::stderr().is_terminal() {
        true => write!(err, "{}", text::to_ansi(std::slice::from_ref(line))),
        false => writeln!(err, "{line}"),
    };
}

/// A time the way uv prints it: `174ms`, `2.31s`, `1m 04s`.
pub fn duration(took: Duration) -> String {
    let secs = took.as_secs();
    match secs {
        0 => format!("{}ms", took.as_millis()),
        1..60 => format!("{secs}.{:02}s", took.subsec_millis() / 10),
        _ => format!("{}m {:02}s", secs / 60, secs % 60),
    }
}

/// The spinner row: `⠴ Preparing packages... (62/63)`.
pub fn header(
    frame: usize,
    message: &str,
    count: Option<(usize, usize)>,
    width: usize,
) -> Line<'static> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let tail = count
        .map(|(done, total)| format!(" ({done}/{total})"))
        .unwrap_or_default();
    let room = width.saturating_sub(2 + wrap::width(&tail));
    Line::from(vec![
        format!("{} ", SPINNER[frame % SPINNER.len()]).into(),
        Span::styled(text::clip_row(message, room), dim),
        Span::styled(tail, dim),
    ])
}

/// One download: `litellm ------------ 25.36 MiB/25.97 MiB`. The bar shrinks
/// before the label does, so the row never runs past `width`.
pub fn bar(label: &str, label_width: usize, done: u64, total: u64, width: usize) -> Line<'static> {
    let t = theme::get();
    let bytes = format!(
        "{:>10}/{:<10}",
        text::bytes(done.min(total)),
        text::bytes(total)
    );
    let label_width = label_width.min(width.saturating_sub(bytes.len() + 2));
    let label = text::clip_row(label, label_width);
    let pad = label_width.saturating_sub(wrap::width(&label));
    let fixed = label_width + 2 + bytes.len();
    let bar_width = width.saturating_sub(fixed).min(BAR_WIDTH);
    let filled = match total {
        0 => bar_width,
        _ => (done.min(total) as u128 * bar_width as u128 / total as u128) as usize,
    };
    Line::from(vec![
        Span::styled(
            format!("{label}{} ", " ".repeat(pad)),
            Style::default().add_modifier(Modifier::DIM),
        ),
        Span::styled("-".repeat(filled), Style::default().fg(t.success)),
        Span::styled(
            "-".repeat(bar_width - filled),
            Style::default().add_modifier(Modifier::DIM),
        ),
        format!(" {bytes}").trim_end().to_string().into(),
    ])
}

struct Download {
    id: usize,
    label: String,
    done: u64,
    total: u64,
}

struct State {
    message: String,
    count: Option<(usize, usize)>,
    downloads: Vec<Download>,
    next_id: usize,
    frame: usize,
    drawn: u16,
}

impl State {
    fn rows(&self) -> Vec<Line<'static>> {
        let width = terminal::size()
            .map_or(80, |(w, _)| w as usize)
            .saturating_sub(1);
        let shown = &self.downloads[..self.downloads.len().min(MAX_BARS)];
        let label_width = shown
            .iter()
            .map(|d| wrap::width(&d.label))
            .max()
            .unwrap_or(0)
            .min(LABEL_MAX);
        let mut rows = vec![header(self.frame, &self.message, self.count, width)];
        rows.extend(
            shown
                .iter()
                .map(|d| bar(&d.label, label_width, d.done, d.total, width)),
        );
        rows
    }

    fn clear(&mut self, out: &mut impl Write) {
        if self.drawn > 0 {
            let _ = queue!(
                out,
                cursor::MoveToPreviousLine(self.drawn),
                terminal::Clear(terminal::ClearType::FromCursorDown)
            );
        }
        self.drawn = 0;
    }

    fn draw(&mut self, out: &mut impl Write) {
        self.clear(out);
        let rows = self.rows();
        let _ = write!(out, "{}", text::to_ansi(&rows));
        let _ = out.flush();
        self.drawn = rows.len() as u16;
    }
}

/// The live rows under the output: a spinner header and one bar per running
/// download, redrawn on a background thread. Does nothing when stderr is not a
/// terminal. Dropping it (or [`Live::finish`]) clears the rows.
pub struct Live {
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    ticker: Option<JoinHandle<()>>,
}

impl Live {
    pub fn start(message: &str) -> Self {
        let state = Arc::new(Mutex::new(State {
            message: message.to_string(),
            count: None,
            downloads: Vec::new(),
            next_id: 0,
            frame: 0,
            drawn: 0,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let ticker = io::stderr().is_terminal().then(|| {
            let (state, stop) = (state.clone(), stop.clone());
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    if let Ok(mut s) = state.lock() {
                        s.frame += 1;
                        s.draw(&mut io::stderr().lock());
                    }
                    std::thread::sleep(TICK);
                }
            })
        });
        Self {
            state,
            stop,
            ticker,
        }
    }

    pub fn set_message(&self, message: &str) {
        if let Ok(mut s) = self.state.lock() {
            s.message = message.to_string();
        }
    }

    /// Show `(done/total)` after the message.
    pub fn set_count(&self, done: usize, total: usize) {
        if let Ok(mut s) = self.state.lock() {
            s.count = Some((done, total));
        }
    }

    /// Add a download row; [`Bar::finish`] takes it away.
    pub fn bar(&self, label: &str, total: u64) -> Bar {
        let mut s = self.state.lock().expect("install state poisoned");
        let id = s.next_id;
        s.next_id += 1;
        s.downloads.push(Download {
            id,
            label: label.to_string(),
            done: 0,
            total,
        });
        Bar {
            id,
            state: self.state.clone(),
        }
    }

    /// Print `line` above the live rows so it lands in scrollback.
    pub fn println(&self, line: &Line<'_>) {
        if self.ticker.is_none() {
            return emit(line);
        }
        let Ok(mut s) = self.state.lock() else {
            return;
        };
        let mut err = io::stderr().lock();
        s.clear(&mut err);
        let _ = write!(err, "{}", text::to_ansi(std::slice::from_ref(line)));
        s.draw(&mut err);
    }

    pub fn finish(self) {}
}

impl Drop for Live {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(ticker) = self.ticker.take() {
            let _ = ticker.join();
            if let Ok(mut s) = self.state.lock() {
                let mut err = io::stderr().lock();
                s.clear(&mut err);
                let _ = err.flush();
            }
        }
    }
}

/// One download row in a [`Live`]. Cheap to clone across worker threads.
#[derive(Clone)]
pub struct Bar {
    id: usize,
    state: Arc<Mutex<State>>,
}

impl Bar {
    pub fn inc(&self, bytes: u64) {
        self.update(|d| d.done = d.done.saturating_add(bytes));
    }

    pub fn set_total(&self, total: u64) {
        self.update(|d| d.total = total);
    }

    pub fn finish(&self) {
        if let Ok(mut s) = self.state.lock() {
            s.downloads.retain(|d| d.id != self.id);
        }
    }

    fn update(&self, apply: impl FnOnce(&mut Download)) {
        if let Ok(mut s) = self.state.lock()
            && let Some(d) = s.downloads.iter_mut().find(|d| d.id == self.id)
        {
            apply(d);
        }
    }
}

#[cfg(test)]
#[path = "tests/install_test.rs"]
mod tests;
