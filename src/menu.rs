//! The slash-command list that opens above the composer as soon as the line
//! starts with `/`, filtered by prefix as the user types.

pub const ROWS: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub name: String,
    pub description: String,
}

impl Command {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

#[derive(Default)]
pub struct Menu {
    commands: Vec<Command>,
    matches: Vec<usize>,
    pub cursor: usize,
    open: bool,
}

impl Menu {
    pub fn set_commands(&mut self, commands: Vec<Command>) {
        self.commands = commands;
        self.matches.clear();
        self.cursor = 0;
    }

    pub fn is_open(&self) -> bool {
        self.open && !self.matches.is_empty()
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    /// Open, filter or close the menu from the current composer text.
    pub fn sync(&mut self, text: &str) {
        let Some(rest) = text.strip_prefix('/') else {
            self.open = false;
            return;
        };
        if rest.contains(' ') || rest.contains('\n') {
            self.open = false;
            return;
        }

        let prefix = rest.to_lowercase();
        self.matches = self
            .commands
            .iter()
            .enumerate()
            .filter(|(_, c)| c.name.to_lowercase().starts_with(&prefix))
            .map(|(i, _)| i)
            .collect();
        if self.cursor >= self.matches.len() {
            self.cursor = 0;
        }
        self.open = true;
    }

    pub fn move_by(&mut self, delta: isize) {
        if self.matches.is_empty() {
            return;
        }
        let len = self.matches.len() as isize;
        self.cursor = ((self.cursor as isize + delta).rem_euclid(len)) as usize;
    }

    pub fn picked(&self) -> Option<&Command> {
        if !self.is_open() {
            return None;
        }
        self.commands.get(self.matches[self.cursor])
    }

    /// The window of matches to draw, each with whether it is selected, and how
    /// many are hidden below it.
    pub fn window(&self) -> (Vec<(&Command, bool)>, usize) {
        if !self.is_open() {
            return (Vec::new(), 0);
        }
        let start = self
            .cursor
            .saturating_sub(ROWS / 2)
            .min(self.matches.len().saturating_sub(ROWS));
        let end = (start + ROWS).min(self.matches.len());

        let rows = self.matches[start..end]
            .iter()
            .enumerate()
            .filter_map(|(offset, &index)| {
                self.commands
                    .get(index)
                    .map(|c| (c, start + offset == self.cursor))
            })
            .collect();
        (rows, self.matches.len() - end)
    }

    pub fn height(&self) -> u16 {
        if !self.is_open() {
            return 0;
        }
        let (rows, hidden) = self.window();
        rows.len() as u16 + u16::from(hidden > 0)
    }
}

#[cfg(test)]
#[path = "tests/menu_test.rs"]
mod tests;
