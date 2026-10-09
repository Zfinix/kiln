# kiln

[![CI](https://github.com/Zfinix/kiln/actions/workflows/ci.yml/badge.svg)](https://github.com/Zfinix/kiln/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/Zfinix/kiln/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![ratatui](https://img.shields.io/badge/ratatui-0.30-4fd6e0.svg)](https://ratatui.rs)

Components for building inline terminal apps in Rust on top of
[ratatui](https://ratatui.rs).

Most TUIs take over the whole screen. kiln keeps a small live pane at the
bottom of the terminal and prints everything else into the terminal's own
scrollback, so scrolling, mouse selection, copy, and search all work the way
they do in the shell.

I built these pieces for the [Aster](https://github.com/Zfinix/aster) coding
agent and its chat client, cinder, then pulled them out so other CLIs can use
them without copying files around.

![The kiln demo: a session header, a user message, a streamed markdown reply with a table and highlighted code, and the slash-command menu open under the prompt](https://raw.githubusercontent.com/Zfinix/kiln/main/assets/demo.png)

## Features

- `InlineTerm` draws a live pane under your output and moves finished lines
  into real scrollback. The pane stays under the transcript through resizes
  and terminal reflow.
- `Composer` is a multi-line prompt with word motions, kill commands, history,
  and folding for large pastes.
- `MarkdownStream` renders text while it streams: tables, lists, quotes, and
  highlighted code.
- `cells` has ready-made blocks for messages, replies, errors, tool output,
  task lists, and diffs.
- There is a filterable picker, a slash-command menu, and a queue for
  allow/deny prompts.
- 16 built-in themes. Switching blends the colours over 450 ms.
- `Renderable` answers "how tall?" and "draw" from the same object, so layout
  and drawing never disagree.
- `TuiGuard` puts the terminal back on panic, early return, or normal exit.

## Installation

kiln is not on crates.io yet. Add it from Git:

```toml
[dependencies]
kiln = { git = "https://github.com/Zfinix/kiln", tag = "v0.3.0" }
```

kiln needs Rust 1.88 or newer (edition 2024). `Tui` runs on
[tokio](https://tokio.rs). If you want a synchronous loop, use `InlineTerm`
directly.

## Quick start

```rust,no_run
use kiln::cells;
use kiln::composer::Composer;
use kiln::guard::TuiGuard;
use kiln::render::Renderable;
use kiln::terminal::{Tui, TuiEvent, restore_raw};
use crossterm::event::KeyCode;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _guard = TuiGuard::install(restore_raw);
    let mut tui = Tui::new(3)?;
    let mut composer = Composer::default();

    loop {
        let width = tui.width();
        let (lines, _cursor) = composer.render(width, "Say something");
        tui.draw(lines.desired_height(width), |frame| {
            lines.render(frame.area(), frame.buffer_mut());
        })?;

        match tui.next_event().await {
            TuiEvent::Key(key) => match key.code {
                KeyCode::Esc => break,
                KeyCode::Enter => {
                    let text = composer.take();
                    tui.insert_history(cells::user(&text, width as usize))?;
                }
                KeyCode::Backspace => composer.backspace(),
                KeyCode::Char(c) => composer.insert(c),
                _ => {}
            },
            TuiEvent::Paste(text) => composer.paste(&text),
            TuiEvent::Resize => tui.resized()?,
            TuiEvent::Mouse(_) | TuiEvent::Draw => {}
        }
    }
    Ok(())
}
```

Each message you send goes into scrollback above the prompt and stays there
after the program exits.

## Components

| Module | What it gives you |
|---|---|
| `term` | `InlineTerm`, the inline viewport, and `insert_lines` to print into scrollback |
| `terminal` | `Tui`: raw mode, async events, bracketed paste, and frame scheduling on top of `InlineTerm` |
| `guard` | `TuiGuard`, which restores the terminal on drop and on panic |
| `composer` | `Composer`, the multi-line prompt editor |
| `markdown` | `MarkdownStream` for streamed text, `render` for text you already have |
| `syntax` | a small highlighter for code blocks in any language |
| `cells` | transcript blocks: `user`, `assistant`, `notice`, `error`, `error_box`, `tool`, `plan`, `patch`, `header`, and more |
| `list` | `ListSelectionView`, a filterable picker with an optional hover hook |
| `menu` | `Menu`, the slash-command list, drawn with `Menu::lines` |
| `approval` | `Approvals`, a queue of allow/deny requests, drawn with `Approvals::lines` |
| `status` | `StatusWidget`, a spinner with an activity label and elapsed time |
| `render` | `Renderable`, `Column`, `Inset`, `Insets` |
| `view` | `View`, the trait for modal panes that take the keyboard |
| `theme`, `palettes` | the active theme, the built-in palettes, and animated switching |
| `mark` | half-block logos tinted with the theme gradient |
| `bigtext` | three-row block digits for timers and readouts |
| `spark` | a one-row history chart |
| `progress` | a bar filled with the theme's gradient, to an eighth of a cell |
| `install` | `uv`-style install output: a spinner with a count, a bar per download, then `Installed 3 packages in 174ms` and ` + name==version` lines |
| `keys` | the `q quit · ? help` footer and the full help list |
| `confirm` | a yes/no question, like `gum confirm`, that starts on the safe answer |
| `input` | a one-line field with a character limit and counter, like `gum input` |
| `wrap`, `text` | Unicode-aware wrapping and clipping, count and duration formatting, and `to_ansi` to print cells without a viewport |

## Examples

Every example runs offline except `chat`. Run one with `cargo run --example <name>`.

### demo

A scripted tour: type a message and a reply streams in, then try `/theme`,
`/plan`, and `/diff`. It is the screenshot at the top of this page.

### pick

A one-question picker, like `gum choose`. Pass options as arguments, type to
filter, press a number or enter. The pane draws on the terminal even when
stdout is captured, so it works in scripts:
`choice=$(cargo run -q --example pick -- rust go zig)`.

![A picker filtered to three crates with the second one selected](https://raw.githubusercontent.com/Zfinix/kiln/main/assets/pick.png)

### tasks

A task runner. The plan and a spinner stay live in the pane while steps run.
Finished steps and their output go into scrollback, and a failed step ends in
an error box.

![A task list with two steps done, one in progress and a spinner below it](https://raw.githubusercontent.com/Zfinix/kiln/main/assets/tasks.png)

### approve

Permission prompts. Requests that arrive while one is open wait in a queue,
and the one-time allow is always preselected.

![A permission prompt for a file edit with a tinted diff preview and one more request waiting](https://raw.githubusercontent.com/Zfinix/kiln/main/assets/approve.png)

### markdown

Streams a markdown file into scrollback a few words at a time, the way a model
reply arrives. This one has no async runtime; it drives `InlineTerm` directly.
Pass a path to render your own file: `cargo run --example markdown -- README.md`.

![Rendered markdown with a heading, a list with code spans, a table, a quote and a highlighted code block](https://raw.githubusercontent.com/Zfinix/kiln/main/assets/markdown.png)

### themes

All 16 built-in themes, printed with `text::to_ansi`. It uses no raw mode and
no viewport, which is all a one-shot CLI needs.

![Sixteen theme swatches in two columns, each with a tinted star, colour chips and diff and code samples](https://raw.githubusercontent.com/Zfinix/kiln/main/assets/themes.png)

### install

A pretend package install drawn the way `uv tool install` draws one: a
spinner while resolving, a thin bar per download, then a summary and the
` + name==version` list. It prints to stderr with no raw mode, and falls back
to plain lines when stderr is piped.

### chat

A full chat client for any agent that speaks the
[Agent Client Protocol](https://agentclientprotocol.com). It starts `aster acp`
by default. Use `--agent <command>` to pick another agent and `--theme <name>`
to change the colours.

## Themes

`default`, `light`, `midnight`, `forest`, `dracula`, `catppuccin`, `nord`,
`gruvbox`, `solarized`, `synthwave`, `github-dark`, `monokai`, `one-dark`,
`ocean`, `ember`, `orchid`.

```rust,no_run
if let Some(entry) = kiln::theme::named("nord") {
    kiln::theme::set(entry.theme);
}
```

Every component reads colours from `theme::get()`, so a switch applies
everywhere on the next frame.

## How the viewport works

The terminal owns everything above the pane. kiln never redraws it. When a
block is finished, `InlineTerm` scrolls it into scrollback with a scrolling
region and records how wide each row was printed. After a resize, those widths
tell it where the terminal's reflow left the end of the transcript, so the pane
goes back directly under it with no blank bands or clipped rows. The tests in
`src/tests/term_test.rs` check this against a simulated terminal.

## Built with kiln

- [tock](https://github.com/Zfinix/tock): a pomodoro timer in your terminal
- [pulse](https://github.com/Zfinix/pulse): internet speed in your terminal
- [ship](https://github.com/Zfinix/ship): conventional commits in your terminal

All of them live together in [kiln-tools](https://github.com/Zfinix/kiln-tools).

## Inspired by

- [Codex CLI](https://github.com/openai/codex/tree/main/codex-rs/tui). Its
  TUI is where the shape comes from: an inline viewport that writes finished
  output into real scrollback, a bottom pane with stacked views, and a list
  selection view.
- [Charm](https://charm.sh). `pick` is modelled on
  [gum choose](https://github.com/charmbracelet/gum), and the overall polish
  aims for what [Bubble Tea](https://github.com/charmbracelet/bubbletea) apps
  get right.
- [ratatui](https://ratatui.rs), which kiln is built on. The viewport in
  `src/term.rs` started from its `Terminal`.

## Contributing

Bug reports and pull requests are welcome. Before opening a PR, run:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs the same checks on Linux, macOS, and Windows.

## License

Licensed under the [Apache License, Version 2.0](https://github.com/Zfinix/kiln/blob/main/LICENSE).

`src/term.rs` is derived from ratatui's `Terminal`
([MIT](https://github.com/ratatui/ratatui/blob/main/LICENSE), © the Ratatui
Developers).
