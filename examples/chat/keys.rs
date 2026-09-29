//! Key handling. Enter sends, ctrl+j opens a line, esc interrupts, ctrl+c twice
//! quits. An open approval or picker owns the keyboard until it is answered.

use std::sync::mpsc::Sender;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::model::{App, Scrollback};

pub fn on_key(app: &mut App, key: KeyEvent, turns: &Sender<String>) -> Scrollback {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    if app.approvals.is_open() {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => app.approvals.move_by(-1),
            KeyCode::Down | KeyCode::Char('j') => app.approvals.move_by(1),
            KeyCode::Enter | KeyCode::Char('y') => {
                let answered = app.approvals.answer();
                return app.decide(answered);
            }
            KeyCode::Esc | KeyCode::Char('n') => {
                let answered = app.approvals.reject();
                return app.decide(answered);
            }
            _ => {}
        }
        return Vec::new();
    }

    if let Some(view) = &mut app.view {
        view.handle_key(key);
        if view.is_complete() {
            app.view = None;
        }
        return Vec::new();
    }

    // Enter runs the highlighted command; tab only completes it, for when the
    // command still needs an argument.
    if let Some(picked) = app.menu.picked() {
        let name = picked.name.clone();
        match key.code {
            KeyCode::Enter => {
                app.composer.clear();
                app.composer.insert_str(&format!("/{name}"));
                return app.submit(turns);
            }
            KeyCode::Tab => {
                app.composer.clear();
                app.composer.insert_str(&format!("/{name} "));
                app.menu.close();
                return Vec::new();
            }
            KeyCode::Up => {
                app.menu.move_by(-1);
                return Vec::new();
            }
            KeyCode::Down => {
                app.menu.move_by(1);
                return Vec::new();
            }
            KeyCode::Esc => {
                app.menu.close();
                return Vec::new();
            }
            _ => {}
        }
    }

    if !(ctrl && key.code == KeyCode::Char('c')) {
        app.quit_armed = false;
    }

    let width = app.width;
    match key.code {
        // The first ctrl+c interrupts or arms, the second leaves, so one stray
        // keystroke never drops a session.
        KeyCode::Char('c') if ctrl => {
            if app.working {
                let _ = app.client.cancel();
                app.working = false;
                app.flash = Some("stopped".into());
            } else if app.quit_armed {
                app.should_quit = true;
            } else {
                app.quit_armed = true;
                app.flash = Some("ctrl+c again to quit".into());
            }
            return Vec::new();
        }
        KeyCode::Esc if app.working => {
            let _ = app.client.cancel();
            app.flash = Some("stopped".into());
            return Vec::new();
        }
        KeyCode::Esc => app.composer.clear(),

        KeyCode::Enter if ctrl || alt => app.composer.insert('\n'),
        KeyCode::Char('j') if ctrl => app.composer.insert('\n'),
        KeyCode::BackTab => app.composer.insert('\n'),
        KeyCode::Enter => return app.submit(turns),

        KeyCode::Char('t') if ctrl => {
            app.show_work = !app.show_work;
            let state = if app.show_work { "shown" } else { "hidden" };
            app.flash = Some(format!("thinking {state}"));
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
        KeyCode::Left if ctrl || alt => app.composer.word_left(),
        KeyCode::Right if ctrl || alt => app.composer.word_right(),
        KeyCode::Left => app.composer.left(),
        KeyCode::Right => app.composer.right(),
        KeyCode::Home => app.composer.home(),
        KeyCode::End => app.composer.end(),
        KeyCode::Char('a') if ctrl => app.composer.home(),
        KeyCode::Char('e') if ctrl => app.composer.end(),
        KeyCode::Char('u') if ctrl => app.composer.kill_to_start(),
        KeyCode::Char('k') if ctrl => app.composer.kill_to_end(),
        KeyCode::Char('w') if ctrl => app.composer.delete_word_back(),
        KeyCode::Backspace if ctrl || alt => app.composer.delete_word_back(),
        KeyCode::Backspace => app.composer.backspace(),
        KeyCode::Delete => app.composer.delete(),

        KeyCode::Char(c) if !ctrl => app.composer.insert(c),
        _ => {}
    }

    app.menu.sync(app.composer.text());
    Vec::new()
}
