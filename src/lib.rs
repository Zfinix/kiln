//! Components for inline terminal apps on ratatui: a viewport that leaves
//! finished output in the terminal's own scrollback, a composer, streaming
//! markdown, pickers, transcript cells, and a themed palette behind them all.

#![doc = include_str!("../README.md")]

pub mod approval;
pub mod cells;
pub mod composer;
pub mod guard;
pub mod list;
pub mod mark;
pub mod markdown;
pub mod menu;
pub mod palettes;
pub mod render;
pub mod status;
pub mod syntax;
pub mod term;
pub mod terminal;
pub mod text;
pub mod theme;
pub mod view;
pub mod wrap;

pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
