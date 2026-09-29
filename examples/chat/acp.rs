//! An Agent Client Protocol client that speaks JSON-RPC to an agent over stdio.
//! Threads and channels, no async runtime: one reader thread, one caller.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use kiln::approval::{Choice as Answer, ChoiceKind, Request};
use kiln::cells::StepStatus;
use kiln::menu::Command as SlashCommand;
use serde_json::{Value, json};

const PROTOCOL_VERSION: u64 = 1;

/// Everything the agent tells us mid-turn, in arrival order.
#[derive(Debug, Clone)]
pub enum Event {
    Thought(String),
    Message(String),
    ToolStarted {
        id: String,
        title: String,
        name: String,
        args: Value,
    },
    ToolFinished {
        id: String,
        status: String,
        output: String,
        patch: Option<Patch>,
    },
    Plan(Vec<(StepStatus, String)>),
    Commands(Vec<SlashCommand>),
    Title(String),
    /// A tool needs a decision, answered later with `Client::decide`. The
    /// payload is the JSON-RPC id the answer goes back under.
    Permission(Box<Request<Value>>),
    TurnEnded(Option<String>),
    Closed,
}

/// A file edit a tool reported, as a unified-style body of `-` and `+` rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    pub path: String,
    pub created: bool,
    pub body: String,
}

#[derive(Debug, Clone, Default)]
pub struct Mode {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct Modes {
    pub current: String,
    pub available: Vec<Mode>,
}

/// A `select` the agent exposes over ACP: provider, model, effort, mode.
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub id: String,
    pub name: String,
    pub current: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, Default)]
pub struct Choice {
    pub value: String,
    pub name: String,
}

type Pending = Arc<Mutex<HashMap<u64, Sender<Value>>>>;

pub struct Client {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    pending: Pending,
    next_id: Mutex<u64>,
    session: String,
    pub modes: Mutex<Modes>,
    configs: Mutex<Vec<Config>>,
    pub agent: String,
}

impl Client {
    /// Start the agent and complete the handshake. The event receiver is handed
    /// back separately: it is Send but not Sync, so it cannot live in a shared
    /// client.
    pub fn dial(bin: &str, args: &[&str], cwd: &str) -> Result<(Self, Receiver<Event>)> {
        let mut child = Command::new(bin)
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("could not start {bin}, is it installed and on your PATH?"))?;

        let stdin = child.stdin.take().context("no stdin")?;
        let stdout = child.stdout.take().context("no stdout")?;

        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let (tx, events) = channel();
        spawn_reader(stdout, pending.clone(), tx);

        let mut client = Self {
            child,
            stdin: Arc::new(Mutex::new(stdin)),
            pending,
            next_id: Mutex::new(0),
            session: String::new(),
            modes: Mutex::new(Modes::default()),
            configs: Mutex::new(Vec::new()),
            agent: "agent".into(),
        };

        let init = client.call(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "clientCapabilities": {
                    "fs": { "readTextFile": false, "writeTextFile": false },
                    "terminal": false
                }
            }),
        )?;
        let name = init["agentInfo"]["name"].as_str().unwrap_or("agent");
        let version = init["agentInfo"]["version"].as_str().unwrap_or("");
        client.agent = format!("{name} {version}").trim().to_string();

        let created = client.call("session/new", json!({ "cwd": cwd, "mcpServers": [] }))?;
        client.session = created["sessionId"]
            .as_str()
            .context("the agent did not start a session")?
            .to_string();

        let modes = &created["modes"];
        let available = modes["availableModes"]
            .as_array()
            .map(|list| {
                list.iter()
                    .map(|m| Mode {
                        id: str_of(&m["id"]),
                        name: str_of(&m["name"]),
                    })
                    .collect()
            })
            .unwrap_or_default();
        *client.modes.lock().unwrap() = Modes {
            current: str_of(&modes["currentModeId"]),
            available,
        };
        *client.configs.lock().unwrap() = parse_configs(&created["configOptions"]);
        Ok((client, events))
    }

    /// Send one message. Blocks until the agent stops, so callers run it on a
    /// worker thread and read `events` for the streaming output.
    pub fn prompt(&self, text: &str) -> Result<String> {
        let res = self.call(
            "session/prompt",
            json!({
                "sessionId": self.session,
                "prompt": [{ "type": "text", "text": text }]
            }),
        )?;
        Ok(str_of(&res["stopReason"]))
    }

    /// Answer a pending permission request.
    pub fn decide(&self, id: &Value, option: &str) -> Result<()> {
        write_line(
            &self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "outcome": { "outcome": "selected", "optionId": option } }
            }),
        )
    }

    /// Answer a permission request without picking any of its options.
    pub fn dismiss(&self, id: &Value) -> Result<()> {
        write_line(
            &self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "outcome": { "outcome": "cancelled" } }
            }),
        )
    }

    pub fn cancel(&self) -> Result<()> {
        write_line(
            &self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "session/cancel",
                "params": { "sessionId": self.session }
            }),
        )
    }

    pub fn set_mode(&self, id: &str) -> Result<()> {
        self.call(
            "session/set_mode",
            json!({ "sessionId": self.session, "modeId": id }),
        )?;
        self.modes.lock().unwrap().current = id.to_string();
        Ok(())
    }

    /// Change one of the agent's select options and remember the new value.
    pub fn set_config(&self, id: &str, value: &str) -> Result<()> {
        self.call(
            "session/set_config_option",
            json!({ "sessionId": self.session, "optionId": id, "value": value }),
        )?;
        if let Ok(mut configs) = self.configs.lock()
            && let Some(config) = configs.iter_mut().find(|c| c.id == id)
        {
            config.current = value.to_string();
        }
        Ok(())
    }

    /// One config option by id, cloned so the caller holds no lock.
    pub fn config(&self, id: &str) -> Option<Config> {
        self.configs
            .lock()
            .ok()?
            .iter()
            .find(|c| c.id == id)
            .cloned()
    }

    pub fn config_ids(&self) -> Vec<(String, String)> {
        self.configs
            .lock()
            .map(|configs| {
                configs
                    .iter()
                    .map(|c| (c.id.clone(), c.name.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn mode_name(&self) -> String {
        let modes = self.modes.lock().unwrap();
        modes
            .available
            .iter()
            .find(|m| m.id == modes.current)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| "auto".into())
    }

    fn call(&self, method: &str, params: Value) -> Result<Value> {
        let id = {
            let mut n = self.next_id.lock().unwrap();
            *n += 1;
            *n
        };
        let (tx, rx) = channel();
        self.pending.lock().unwrap().insert(id, tx);

        write_line(
            &self.stdin,
            &json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }),
        )?;

        let res = rx
            .recv()
            .map_err(|_| anyhow!("the agent stopped responding"))?;
        if let Some(err) = res.get("error") {
            return Err(anyhow!(str_of(&err["message"])));
        }
        Ok(res["result"].clone())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn write_line(stdin: &Arc<Mutex<ChildStdin>>, value: &Value) -> Result<()> {
    let mut out = stdin.lock().unwrap();
    writeln!(out, "{value}")?;
    out.flush()?;
    Ok(())
}

/// Only `select` options are read: they are the ones a list can present.
fn parse_configs(value: &Value) -> Vec<Config> {
    value
        .as_array()
        .map(|list| {
            list.iter()
                .filter(|c| c["type"] == json!("select"))
                .map(|c| Config {
                    id: str_of(&c["id"]),
                    name: str_of(&c["name"]),
                    current: str_of(&c["currentValue"]),
                    choices: c["options"]
                        .as_array()
                        .map(|opts| {
                            opts.iter()
                                .map(|o| Choice {
                                    value: str_of(&o["value"]),
                                    name: str_of(&o["name"]),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn str_of(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

fn spawn_reader(stdout: ChildStdout, pending: Pending, tx: Sender<Event>) {
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let line = line.trim();
            if !line.starts_with('{') {
                continue;
            }
            let Ok(msg) = serde_json::from_str::<Value>(line) else {
                continue;
            };

            match msg["method"].as_str() {
                Some("session/update") => {
                    for event in translate(&msg["params"]["update"]) {
                        if tx.send(event).is_err() {
                            return;
                        }
                    }
                }
                Some("session/request_permission") => {
                    let _ = tx.send(Event::Permission(Box::new(permission(&msg))));
                }
                Some(_) => {}
                None => {
                    if let Some(id) = msg["id"].as_u64()
                        && let Some(reply) = pending.lock().unwrap().remove(&id)
                    {
                        let _ = reply.send(msg);
                    }
                }
            }
        }
        // Dropping the pending senders unblocks every in-flight call.
        pending.lock().unwrap().clear();
        let _ = tx.send(Event::Closed);
    });
}

/// A permission request as a kiln approval. Nothing is decided here: the agent
/// waits until the user answers. Options of a kind we do not know are dropped
/// rather than guessed at.
fn permission(msg: &Value) -> Request<Value> {
    let params = &msg["params"];
    let call = &params["toolCall"];
    let raw = &call["rawInput"];

    let preview = if let Some(command) = raw["command"].as_str() {
        let mut line = command.to_string();
        for arg in raw["args"].as_array().into_iter().flatten() {
            if let Some(arg) = arg.as_str() {
                line.push(' ');
                line.push_str(arg);
            }
        }
        vec![line]
    } else if let Some(content) = call["content"].as_array() {
        content
            .iter()
            .flat_map(|c| {
                str_of(&c["content"]["text"])
                    .lines()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .collect()
    } else {
        raw["path"]
            .as_str()
            .map(str::to_string)
            .into_iter()
            .collect()
    };

    let choices = params["options"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|o| {
            let kind = match o["kind"].as_str()? {
                "allow_once" => ChoiceKind::AllowOnce,
                "allow_always" => ChoiceKind::AllowAlways,
                "reject_once" => ChoiceKind::RejectOnce,
                "reject_always" => ChoiceKind::RejectAlways,
                _ => return None,
            };
            Some(Answer {
                id: str_of(&o["optionId"]),
                label: str_of(&o["name"]),
                kind,
            })
        })
        .collect();

    Request {
        title: str_of(&call["title"]),
        preview,
        choices,
        payload: msg["id"].clone(),
    }
}

fn translate(update: &Value) -> Vec<Event> {
    let text = |v: &Value| str_of(&v["content"]["text"]);

    match update["sessionUpdate"].as_str().unwrap_or_default() {
        "agent_thought_chunk" => vec![Event::Thought(text(update))],
        "agent_message_chunk" => vec![Event::Message(text(update))],
        "tool_call" => vec![Event::ToolStarted {
            id: str_of(&update["toolCallId"]),
            title: str_of(&update["title"]),
            name: str_of(&update["name"]),
            args: update["rawInput"].clone(),
        }],
        "tool_call_update" => vec![Event::ToolFinished {
            id: str_of(&update["toolCallId"]),
            status: str_of(&update["status"]),
            output: tool_output(update),
            patch: update["content"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|b| b["type"] == "diff")
                .map(|b| {
                    let (old, new) = (str_of(&b["oldText"]), str_of(&b["newText"]));
                    let old: Vec<&str> = old.lines().collect();
                    let new: Vec<&str> = new.lines().collect();
                    let head = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
                    let tail = old[head..]
                        .iter()
                        .rev()
                        .zip(new[head..].iter().rev())
                        .take_while(|(a, b)| a == b)
                        .count();
                    let removed = old[head..old.len() - tail].iter().map(|l| format!("-{l}"));
                    let added = new[head..new.len() - tail].iter().map(|l| format!("+{l}"));
                    Patch {
                        path: str_of(&b["path"]),
                        created: old.is_empty(),
                        body: removed.chain(added).collect::<Vec<_>>().join("\n"),
                    }
                }),
        }],
        "plan" => {
            let steps = update["entries"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|e| {
                    let status = match e["status"].as_str().unwrap_or_default() {
                        "completed" => StepStatus::Done,
                        "in_progress" => StepStatus::InProgress,
                        _ => StepStatus::Pending,
                    };
                    (status, str_of(&e["content"]))
                })
                .collect();
            vec![Event::Plan(steps)]
        }
        "available_commands_update" => {
            let commands = update["availableCommands"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|c| SlashCommand::new(str_of(&c["name"]), str_of(&c["description"])))
                .collect();
            vec![Event::Commands(commands)]
        }
        "session_info_update" => vec![Event::Title(str_of(&update["title"]))],
        _ => Vec::new(),
    }
}

/// Flatten the text blocks a tool call carries, falling back to the raw output
/// when it reports none. Colour escapes are stripped, since the transcript
/// styles output itself.
fn tool_output(update: &Value) -> String {
    let joined: String = update["content"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|b| str_of(&b["content"]["text"]))
        .collect();
    let raw = match joined.is_empty() {
        true => str_of(&update["rawOutput"]),
        false => joined,
    };

    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        if chars.next() == Some('[') {
            for c in chars.by_ref() {
                if ('@'..='~').contains(&c) {
                    break;
                }
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "acp_test.rs"]
mod tests;
