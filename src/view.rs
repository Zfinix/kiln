//! A modal view shown over the composer. Only the top view of a stack gets keys.

use ratatui::crossterm::event::KeyEvent;

use crate::render::Renderable;

/// A modal that takes over the keyboard until it reports itself complete.
/// Implementors draw through [`Renderable`], handle keys, and send their
/// result through whatever channel they were built with.
pub trait View: Renderable {
    /// React to a key press.
    fn handle_key(&mut self, key: KeyEvent);

    /// True once the view is done and should be closed.
    fn is_complete(&self) -> bool;

    /// A click on `row` of the view. Return true if it was handled.
    fn handle_click(&mut self, _row: u16) -> bool {
        false
    }

    /// A wheel scroll, in rows.
    fn handle_scroll(&mut self, _delta: isize) {}

    /// A bracketed paste. Return true if it was handled.
    fn handle_paste(&mut self, _text: String) -> bool {
        false
    }
}
