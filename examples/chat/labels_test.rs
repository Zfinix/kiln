use super::*;
use serde_json::json;

#[test]
fn matches_aster_phrasing() {
    let cases = [
        ("read_file", json!({"path": "main.rs"}), "Read main.rs"),
        ("read_file", json!({}), "Read file"),
        ("list_files", json!({}), "Listed the project root"),
        ("list_files", json!({"dir": "src"}), "Listed src"),
        (
            "search_files",
            json!({"query": "todo"}),
            "Searched \u{201c}todo\u{201d}",
        ),
        (
            "find_files",
            json!({"pattern": "*.rs"}),
            "Found files matching *.rs",
        ),
        (
            "run_command",
            json!({"description": "Build it"}),
            "Build it",
        ),
        (
            "run_command",
            json!({"command": "cargo", "args": ["test", "--all"]}),
            "Ran cargo test --all",
        ),
        ("run_command", json!({}), "Ran a command"),
        ("some_mcp_tool", json!({}), "Some mcp tool"),
    ];

    for (name, args, want) in cases {
        assert_eq!(step_label(name, &args), want, "for {name}");
    }
}
