//! A yes/no question, like `gum confirm`: arrows or tab to move, `y`/`n` to
//! answer at once, enter to take the highlighted answer, esc for no.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::render::Renderable;
use crate::theme;
use crate::view::View;

/// The question and, once given, the answer. Starts on "No" unless built
/// with [`Self::default_yes`], so a stray enter never does the risky thing.
pub struct Confirm {
    prompt: String,
    yes: bool,
    answer: Option<bool>,
}

impl Confirm {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            yes: false,
            answer: None,
        }
    }

    /// Highlight "Yes" first, for questions where yes is the safe answer.
    pub fn default_yes(mut self) -> Self {
        self.yes = true;
        self
    }

    /// `Some` once the user has answered.
    pub fn answer(&self) -> Option<bool> {
        self.answer
    }

    fn line(&self) -> Line<'static> {
        let t = theme::get();
        let choice = |label: &'static str, on: bool| match on {
            true => Span::styled(format!(" {label} "), t.selected_style()),
            false => Span::styled(format!(" {label} "), t.dim_style()),
        };
        Line::from(vec![
            Span::styled(format!("{} ", self.prompt), t.text_style()),
            choice("Yes", self.yes),
            Span::raw(" "),
            choice("No", !self.yes),
        ])
    }
}

impl Renderable for Confirm {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.line().render(area, buf);
    }
    fn desired_height(&self, _width: u16) -> u16 {
        1
    }
}

impl View for Confirm {
    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
                self.yes = !self.yes;
            }
            KeyCode::Char('y' | 'Y') => self.answer = Some(true),
            KeyCode::Char('n' | 'N') | KeyCode::Esc => self.answer = Some(false),
            KeyCode::Enter => self.answer = Some(self.yes),
            _ => {}
        }
    }

    fn is_complete(&self) -> bool {
        self.answer.is_some()
    }
}

#[cfg(test)]
#[path = "tests/confirm_test.rs"]
mod tests;
