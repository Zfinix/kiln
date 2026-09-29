//! Application state and the update step. Nothing here draws; nothing here
//! talks to the terminal. Every update returns finished lines for scrollback.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::Sender;

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use serde_json::Value;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use kiln::approval::{Approvals, Choice, Request};
use kiln::cells::{self, StepStatus};
use kiln::composer::Composer;
use kiln::list::{ListSelectionView, SelectionItem};
use kiln::markdown::MarkdownStream;
use kiln::menu::{Command, Menu};
use kiln::view::View;
use kiln::{theme, wrap};

use crate::acp::{Client, Event};
use crate::labels::step_label;

/// Past this many changed rows an edit is summarised as one step instead of
/// printed whole, so a new file cannot flood the scrollback.
const MAX_PATCH_LINES: usize = 40;

pub type Scrollback = Vec<Line<'static>>;

/// What a picker sends back when the user settles on a row.
#[derive(Clone)]
pub enum Pick {
    Setting {
        id: String,
        value: String,
        name: String,
    },
    Theme(String),
}

#[derive(Default)]
struct Step {
    label: String,
    output: String,
}

pub struct App {
    pub client: Arc<Client>,
    pub composer: Composer,
    pub menu: Menu,
    pub approvals: Approvals<Value>,
    pub view: Option<Box<dyn View>>,
    picks: UnboundedReceiver<Pick>,
    picks_tx: UnboundedSender<Pick>,
    stream: MarkdownStream,
    thought: String,
    answering: bool,
    grouped: bool,
    steps: HashMap<String, Step>,
    skills: Vec<Command>,
    plan: Vec<(StepStatus, String)>,

    pub agent: String,
    pub mode_name: String,
    pub theme_name: String,
    pub title: Option<String>,
    pub flash: Option<String>,

    pub width: u16,
    pub working: bool,
    pub thinking: bool,
    pub show_work: bool,
    pub frame: usize,
    pub quit_armed: bool,
    pub should_quit: bool,
}

impl App {
    pub fn new(client: Arc<Client>, theme_name: String, width: u16) -> Self {
        let (picks_tx, picks) = unbounded_channel();
        let mut app = Self {
            agent: client.agent.clone(),
            mode_name: client.mode_name(),
            client,
            composer: Composer::default(),
            menu: Menu::default(),
            approvals: Approvals::default(),
            view: None,
            picks,
            picks_tx,
            stream: MarkdownStream::default(),
            thought: String::new(),
            answering: false,
            grouped: false,
            steps: HashMap::new(),
            skills: Vec::new(),
            plan: Vec::new(),
            theme_name,
            title: None,
            flash: None,
            width,
            working: false,
            thinking: false,
            show_work: true,
            frame: 0,
            quit_armed: false,
            should_quit: false,
        };
        app.set_width(width);
        app.menu.set_commands(app.commands());
        app
    }

    pub fn set_width(&mut self, width: u16) {
        self.width = width;
        self.stream.set_width((width as usize).saturating_sub(4));
    }

    /// The agent's settings first, then the commands answered here, then the
    /// agent's skills. The agent only advertises skills, so without this the
    /// settings it does expose would be invisible in the menu.
    fn commands(&self) -> Vec<Command> {
        let mut all: Vec<Command> = self
            .client
            .config_ids()
            .into_iter()
            .map(|(id, name)| Command::new(id, format!("Change the {}", name.to_lowercase())))
            .collect();
        all.push(Command::new("theme", "Switch the colour theme"));
        all.push(Command::new("quit", "Leave kiln chat"));
        all.extend(self.skills.iter().cloned());
        all
    }

    /// Apply whatever the open picker settled on.
    pub fn take_picks(&mut self) -> Scrollback {
        let mut lines = Vec::new();
        while let Ok(pick) = self.picks.try_recv() {
            match pick {
                Pick::Setting { id, value, name } => {
                    if let Err(e) = self.client.set_config(&id, &value) {
                        lines.extend(
                            self.note_error(&format!("Could not change the {id}. Try again.\n{e}")),
                        );
                        continue;
                    }
                    if id == "mode" {
                        self.mode_name = name.clone();
                    }
                    self.flash = Some(format!("{id} {}", name.to_lowercase()));
                }
                Pick::Theme(name) => {
                    if let Some(entry) = theme::named(&name) {
                        theme::set(entry.theme);
                        self.flash = Some(format!("theme {name}"));
                        self.theme_name = name;
                    }
                }
            }
        }
        lines
    }

    /// Answer the approval the user just settled and report it for scrollback.
    pub fn decide(&mut self, answered: Option<(Request<Value>, Choice)>) -> Scrollback {
        let Some((request, choice)) = answered else {
            return Vec::new();
        };
        if let Err(e) = self.client.decide(&request.payload, &choice.id) {
            return self.note_error(&format!(
                "Your answer did not reach the agent. Try the request again.\n{e}"
            ));
        }
        let t = theme::get();
        let (mark, style) = match choice.kind.allows() {
            true => ("✓ ", Style::default().fg(t.success)),
            false => ("✗ ", t.error_style()),
        };
        let line = Line::from(vec![
            Span::styled(mark, style),
            Span::styled(request.title, t.dim_style()),
            Span::styled(format!("  {}", choice.label), t.dimmer_style()),
        ]);
        wrap::wrap_line(line, self.width as usize)
    }

    /// Fold one agent event into the app, returning lines for scrollback.
    pub fn on_event(&mut self, event: Event) -> Scrollback {
        let width = self.width as usize;
        match event {
            Event::Message(text) => {
                self.thinking = false;
                let mut lines = self.flush_thought();
                let answer = self.stream.push(&text);
                if !answer.is_empty() {
                    lines.extend(cells::assistant(answer, !self.answering, width));
                    self.answering = true;
                    self.grouped = false;
                }
                lines
            }
            Event::Thought(text) => {
                self.thinking = true;
                self.thought.push_str(&text);
                Vec::new()
            }
            Event::ToolStarted {
                id,
                title,
                name,
                args,
            } => {
                let label = match title.is_empty() {
                    true => step_label(&name, &args),
                    false => title,
                };
                self.steps.insert(
                    id,
                    Step {
                        label,
                        output: String::new(),
                    },
                );
                self.flush_thought()
            }
            Event::ToolFinished {
                id,
                status,
                output,
                patch,
            } => {
                let failed = match status.as_str() {
                    "failed" => true,
                    "completed" | "" => false,
                    _ => {
                        if let Some(step) = self.steps.get_mut(&id)
                            && !output.is_empty()
                        {
                            step.output = output;
                        }
                        return Vec::new();
                    }
                };
                let step = self.steps.remove(&id).unwrap_or_default();
                let output = match output.is_empty() {
                    true => step.output,
                    false => output,
                };
                let mut lines = self.flush_thought();
                match patch {
                    Some(patch) if !failed && patch.body.lines().count() <= MAX_PATCH_LINES => {
                        let verb = if patch.created { "Created" } else { "Edited" };
                        lines.extend(cells::patch(verb, &patch.path, &patch.body, width));
                        self.grouped = false;
                    }
                    _ if !failed && output.trim().is_empty() => {
                        lines.extend(cells::group_row("Steps", &step.label, self.grouped, width));
                        self.grouped = true;
                    }
                    _ => {
                        lines.extend(cells::tool(&step.label, output.trim_end(), failed, width));
                        self.grouped = false;
                    }
                }
                lines
            }
            Event::Plan(steps) => {
                if steps == self.plan {
                    return Vec::new();
                }
                let mut lines = self.flush_thought();
                lines.extend(cells::plan(&steps, width));
                self.plan = steps;
                self.grouped = false;
                lines
            }
            Event::Commands(commands) => {
                self.skills = commands;
                self.menu.set_commands(self.commands());
                Vec::new()
            }
            Event::Title(title) => {
                self.title = Some(title);
                Vec::new()
            }
            Event::Permission(request) if request.choices.is_empty() => {
                let _ = self.client.dismiss(&request.payload);
                self.note_error(&format!(
                    "{} asked to {}, but offered no answers kiln chat understands, so it was declined.",
                    self.agent, request.title
                ))
            }
            Event::Permission(request) => {
                self.approvals.push(*request);
                Vec::new()
            }
            Event::TurnEnded(error) => {
                self.working = false;
                self.thinking = false;
                let mut lines = self.flush_thought();
                lines.extend(cells::assistant(
                    self.stream.flush(),
                    !self.answering,
                    width,
                ));
                if let Some(message) = error {
                    lines.extend(cells::error(
                        &format!("The agent could not finish that reply. Try sending it again.\n{message}"),
                        width,
                    ));
                }
                self.answering = false;
                self.grouped = false;
                lines
            }
            Event::Closed => {
                self.working = false;
                self.note_error(
                    "The agent stopped running. Quit and start kiln chat again to keep going.",
                )
            }
        }
    }

    /// Reasoning is held until something else arrives, then printed whole;
    /// ctrl+t decides whether it is shown or folded to a one-line count.
    fn flush_thought(&mut self) -> Scrollback {
        let thought = std::mem::take(&mut self.thought);
        let lines = cells::reasoning(&thought, self.show_work, self.width as usize);
        if !lines.is_empty() {
            self.grouped = false;
        }
        lines
    }

    /// Send the composer, or run it here when it is one of the commands the
    /// example answers itself.
    pub fn submit(&mut self, turns: &Sender<String>) -> Scrollback {
        if self.composer.text().trim().is_empty() || self.working {
            return Vec::new();
        }
        let mut text = self.composer.take();
        for (mark, path) in self.composer.take_refs() {
            text = text.replace(&mark, &path);
        }
        let text = text.trim().to_string();
        self.menu.close();

        if let Some(rest) = text.strip_prefix('/') {
            let (name, arg) = rest.split_once(' ').unwrap_or((rest, ""));
            let arg = arg.trim();
            match name {
                "theme" if arg.is_empty() => {
                    let items = theme::all()
                        .iter()
                        .map(|entry| SelectionItem {
                            name: entry.name.clone(),
                            description: entry.description.clone(),
                            is_current: entry.name == self.theme_name,
                            event: Pick::Theme(entry.name.clone()),
                        })
                        .collect();
                    let restore = Some(Pick::Theme(self.theme_name.clone()));
                    let view =
                        ListSelectionView::new("Theme", items, self.picks_tx.clone(), restore)
                            .with_on_hover(Box::new(|item: &SelectionItem<Pick>| {
                                if let Some(entry) = theme::named(&item.name) {
                                    theme::set(entry.theme);
                                }
                            }));
                    self.view = Some(Box::new(view));
                    return Vec::new();
                }
                "theme" => {
                    return match theme::named(arg) {
                        Some(entry) => {
                            theme::set(entry.theme);
                            self.theme_name = entry.name.clone();
                            self.flash = Some(format!("theme {arg}"));
                            Vec::new()
                        }
                        None => self.note_error(&format!(
                            "There is no theme called {arg}. Type /theme to pick from the list."
                        )),
                    };
                }
                "mode" if !arg.is_empty() => {
                    let modes = self.client.modes.lock().unwrap().clone();
                    let found = modes.available.iter().find(|m| {
                        m.name.eq_ignore_ascii_case(arg) || m.id.eq_ignore_ascii_case(arg)
                    });
                    let Some(mode) = found else {
                        let names: Vec<String> = modes
                            .available
                            .iter()
                            .map(|m| m.name.to_lowercase())
                            .collect();
                        return self.note_error(&format!(
                            "There is no mode called {arg}. Try one of: {}",
                            names.join(", ")
                        ));
                    };
                    return match self.client.set_mode(&mode.id) {
                        Ok(()) => {
                            self.mode_name = mode.name.clone();
                            self.flash = Some(format!("mode {}", mode.name.to_lowercase()));
                            Vec::new()
                        }
                        Err(e) => self.note_error(&format!(
                            "Could not switch to {arg} mode. Try again.\n{e}"
                        )),
                    };
                }
                "quit" | "exit" => {
                    self.should_quit = true;
                    return Vec::new();
                }
                id => {
                    if let Some(config) = self.client.config(id) {
                        let items = config
                            .choices
                            .iter()
                            .map(|choice| SelectionItem {
                                name: choice.name.clone(),
                                description: String::new(),
                                is_current: choice.value == config.current,
                                event: Pick::Setting {
                                    id: config.id.clone(),
                                    value: choice.value.clone(),
                                    name: choice.name.clone(),
                                },
                            })
                            .collect();
                        self.view = Some(Box::new(ListSelectionView::new(
                            config.name,
                            items,
                            self.picks_tx.clone(),
                            None,
                        )));
                        return Vec::new();
                    }
                }
            }
        }

        self.working = true;
        self.flash = None;
        let lines = cells::user(&text, self.width as usize);
        let _ = turns.send(text);
        lines
    }

    fn note_error(&self, text: &str) -> Scrollback {
        cells::error(text, self.width as usize)
    }
}
