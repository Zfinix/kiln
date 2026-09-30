//! A one-line text field, like `gum input`: a prompt, a placeholder, an
//! optional character limit with a counter, and emacs-style editing keys.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::render::Renderable;
use crate::view::View;
use crate::{theme, wrap};

/// How the field was closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Submitted,
    Cancelled,
}

pub struct Input {
    prompt: String,
    placeholder: String,
    value: String,
    cursor: usize,
    limit: usize,
    outcome: Option<Outcome>,
}

impl Input {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            placeholder: String::new(),
            value: String::new(),
            cursor: 0,
            limit: usize::MAX,
            outcome: None,
        }
    }

    /// Dim text shown while the field is empty.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Refuse input past `chars` characters and show a counter.
    pub fn limit(mut self, chars: usize) -> Self {
        self.limit = chars;
        self
    }

    /// Start with `text` in the field, caret at the end.
    pub fn value(mut self, text: impl Into<String>) -> Self {
        self.value = text.into();
        self.cursor = self.value.len();
        self
    }

    pub fn text(&self) -> &str {
        &self.value
    }

    /// `Some` once enter or esc has closed the field.
    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    fn insert(&mut self, c: char) {
        if self.value.chars().count() >= self.limit {
            return;
        }
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    fn prev(&self) -> usize {
        self.value[..self.cursor]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i)
    }

    fn next(&self) -> usize {
        self.value[self.cursor..]
            .chars()
            .next()
            .map_or(self.cursor, |c| self.cursor + c.len_utf8())
    }

    fn counter(&self) -> Option<String> {
        (self.limit != usize::MAX)
            .then(|| format!(" {}/{}", self.value.chars().count(), self.limit))
    }

    /// The first byte of `value` that is drawn, so the caret stays in view.
    fn scroll(&self, room: usize) -> usize {
        let mut start = 0;
        while wrap::width(&self.value[start..self.cursor]) >= room && start < self.cursor {
            start += self.value[start..].chars().next().map_or(1, char::len_utf8);
        }
        start
    }

    fn room(&self, width: u16) -> usize {
        let fixed = wrap::width(&self.prompt) + self.counter().map_or(0, |c| wrap::width(&c));
        (width as usize).saturating_sub(fixed).max(1)
    }
}

impl Renderable for Input {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let t = theme::get();
        let mut spans = vec![Span::styled(self.prompt.clone(), t.accent_style())];
        match self.value.is_empty() {
            true => spans.push(Span::styled(self.placeholder.clone(), t.faint_style())),
            false => {
                let start = self.scroll(self.room(area.width));
                spans.push(Span::styled(
                    self.value[start..].to_string(),
                    t.text_style(),
                ));
            }
        }
        let mut line = Line::from(spans);
        if let Some(counter) = self.counter() {
            let full = self.value.chars().count() >= self.limit;
            let style = if full {
                t.error_style()
            } else {
                t.faint_style()
            };
            let used = line.width();
            let pad = (area.width as usize).saturating_sub(used + wrap::width(&counter));
            line.spans.push(Span::raw(" ".repeat(pad)));
            line.spans.push(Span::styled(counter, style));
        }
        line.render(area, buf);
    }

    fn desired_height(&self, _width: u16) -> u16 {
        1
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        let start = self.scroll(self.room(area.width));
        let col = wrap::width(&self.prompt) + wrap::width(&self.value[start..self.cursor]);
        Some((area.x + col as u16, area.y))
    }
}

impl View for Input {
    fn handle_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Enter => self.outcome = Some(Outcome::Submitted),
            KeyCode::Esc => self.outcome = Some(Outcome::Cancelled),
            KeyCode::Char('c') if ctrl => self.outcome = Some(Outcome::Cancelled),
            KeyCode::Char('a') if ctrl => self.cursor = 0,
            KeyCode::Char('e') if ctrl => self.cursor = self.value.len(),
            KeyCode::Char('u') if ctrl => {
                self.value.drain(..self.cursor);
                self.cursor = 0;
            }
            KeyCode::Char('w') if ctrl => {
                let head = self.value[..self.cursor].trim_end();
                let start = head.rfind(char::is_whitespace).map_or(0, |i| i + 1);
                self.value.drain(start..self.cursor);
                self.cursor = start;
            }
            KeyCode::Char(c) if !ctrl => self.insert(c),
            KeyCode::Backspace => {
                let prev = self.prev();
                self.value.drain(prev..self.cursor);
                self.cursor = prev;
            }
            KeyCode::Delete => {
                let next = self.next();
                self.value.drain(self.cursor..next);
            }
            KeyCode::Left => self.cursor = self.prev(),
            KeyCode::Right => self.cursor = self.next(),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.value.len(),
            _ => {}
        }
    }

    fn is_complete(&self) -> bool {
        self.outcome.is_some()
    }

    fn handle_paste(&mut self, text: String) -> bool {
        text.chars()
            .filter(|c| !c.is_control())
            .for_each(|c| self.insert(c));
        true
    }
}

#[cfg(test)]
#[path = "tests/input_test.rs"]
mod tests;
