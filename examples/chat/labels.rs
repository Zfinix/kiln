//! Tool calls read as sentences. Ported from aster's `tui::step_label` so both
//! agents narrate their work with the same words.

use serde_json::Value;

pub fn step_label(name: &str, args: &Value) -> String {
    let s = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default();

    match name {
        "read_file" => with_arg("Read file", "Read", s("path")),
        "list_files" => with_arg("Listed the project root", "Listed", s("dir")),
        "search_files" => format!("Searched \u{201c}{}\u{201d}", s("query")),
        "find_files" => format!("Found files matching {}", s("pattern")),
        "run_command" => match s("description") {
            "" => with_arg("Ran a command", "Ran", &command_line(args)),
            summary => summary.to_string(),
        },
        "edit_file" | "write_file" => with_arg("Edited a file", "Edited", s("path")),
        "apply_patch" => "Applied a patch".to_string(),
        "fetch_url" => with_arg("Fetched a page", "Fetched", s("url")),
        other => humanize(other),
    }
}

fn with_arg(empty: &str, verb: &str, arg: &str) -> String {
    match arg {
        "" => empty.to_string(),
        arg => format!("{verb} {arg}"),
    }
}

/// Rebuild the shell line from the structured arguments.
fn command_line(args: &Value) -> String {
    let Some(cmd) = args.get("command").and_then(Value::as_str) else {
        return String::new();
    };
    let mut parts = vec![cmd.to_string()];
    if let Some(list) = args.get("args").and_then(Value::as_array) {
        parts.extend(list.iter().filter_map(Value::as_str).map(String::from));
    }
    parts.join(" ")
}

/// Fallback for tools with no bespoke phrasing, including MCP tools whose names
/// arrive as snake_case.
fn humanize(name: &str) -> String {
    let mut words = name.split(['_', '-', '.']).filter(|w| !w.is_empty());
    let Some(first) = words.next() else {
        return name.to_string();
    };
    let mut out = first[..1].to_uppercase() + &first[1..];
    for word in words {
        out.push(' ');
        out.push_str(word);
    }
    out
}

#[cfg(test)]
#[path = "labels_test.rs"]
mod tests;
