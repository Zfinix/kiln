//! A queue of permission requests. Extra requests wait behind the current one
//! rather than stacking prompts, and the default answer is always the safe,
//! one-time allow.

use std::collections::VecDeque;

pub const MAX_PREVIEW_ROWS: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
}

impl ChoiceKind {
    pub fn allows(self) -> bool {
        match self {
            ChoiceKind::AllowOnce | ChoiceKind::AllowAlways => true,
            ChoiceKind::RejectOnce | ChoiceKind::RejectAlways => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub id: String,
    pub label: String,
    pub kind: ChoiceKind,
}

/// One request: what it wants to do, the rows to preview, the answers it
/// accepts, and whatever the caller needs to route the answer back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request<T> {
    pub title: String,
    pub preview: Vec<String>,
    pub choices: Vec<Choice>,
    pub payload: T,
}

pub struct Approvals<T> {
    current: Option<Request<T>>,
    queue: VecDeque<Request<T>>,
    pub selected: usize,
}

impl<T> Default for Approvals<T> {
    fn default() -> Self {
        Self {
            current: None,
            queue: VecDeque::new(),
            selected: 0,
        }
    }
}

impl<T> Approvals<T> {
    /// Show `request` now, or park it behind whatever is already waiting.
    pub fn push(&mut self, request: Request<T>) {
        match self.current {
            None => {
                self.selected = default_choice(&request);
                self.current = Some(request);
            }
            Some(_) => self.queue.push_back(request),
        }
    }

    pub fn current(&self) -> Option<&Request<T>> {
        self.current.as_ref()
    }

    pub fn is_open(&self) -> bool {
        self.current.is_some()
    }

    pub fn waiting(&self) -> usize {
        self.queue.len()
    }

    pub fn move_by(&mut self, delta: isize) {
        let Some(request) = &self.current else { return };
        let len = request.choices.len() as isize;
        if len == 0 {
            return;
        }
        self.selected = ((self.selected as isize + delta).rem_euclid(len)) as usize;
    }

    /// Take the selected choice and advance to the next request.
    pub fn answer(&mut self) -> Option<(Request<T>, Choice)> {
        let choice = self.current.as_ref()?.choices.get(self.selected)?.clone();
        let request = self.current.take()?;

        self.current = self.queue.pop_front();
        self.selected = self.current.as_ref().map(default_choice).unwrap_or(0);
        Some((request, choice))
    }

    /// Answer with the one-time rejection, for when the user presses escape.
    pub fn reject(&mut self) -> Option<(Request<T>, Choice)> {
        let request = self.current.as_ref()?;
        self.selected = request
            .choices
            .iter()
            .position(|c| c.kind == ChoiceKind::RejectOnce)
            .or_else(|| request.choices.iter().position(|c| !c.kind.allows()))?;
        self.answer()
    }

    /// Preview rows, capped so a large patch cannot push the composer away. A
    /// preview the title already says is dropped rather than shown twice.
    pub fn preview(&self) -> (Vec<String>, usize) {
        let Some(request) = &self.current else {
            return (Vec::new(), 0);
        };
        if request.preview.len() == 1 && request.title.contains(request.preview[0].trim()) {
            return (Vec::new(), 0);
        }
        let hidden = request.preview.len().saturating_sub(MAX_PREVIEW_ROWS);
        let shown = request
            .preview
            .iter()
            .take(MAX_PREVIEW_ROWS)
            .cloned()
            .collect();
        (shown, hidden)
    }

    /// Rows the prompt needs: title, preview, hidden note, choices and hint.
    pub fn height(&self) -> u16 {
        let Some(request) = &self.current else {
            return 0;
        };
        let (shown, hidden) = self.preview();
        (1 + shown.len() + usize::from(hidden > 0) + request.choices.len() + 1) as u16
    }
}

/// Start on the one-time allow, so the common case is one keystroke, but never
/// preselect a "remember this" choice.
fn default_choice<T>(request: &Request<T>) -> usize {
    request
        .choices
        .iter()
        .position(|c| c.kind == ChoiceKind::AllowOnce)
        .or_else(|| request.choices.iter().position(|c| c.kind.allows()))
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "tests/approval_test.rs"]
mod tests;
