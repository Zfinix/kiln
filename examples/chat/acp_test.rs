use super::*;

#[test]
fn translates_a_message_chunk() {
    let update = json!({
        "sessionUpdate": "agent_message_chunk",
        "content": { "type": "text", "text": "pong" }
    });
    match &translate(&update)[..] {
        [Event::Message(text)] => assert_eq!(text, "pong"),
        other => panic!("want a message, got {other:?}"),
    }
}

#[test]
fn translates_a_tool_call_pair() {
    let started = json!({
        "sessionUpdate": "tool_call",
        "toolCallId": "t1",
        "title": "Echo hi",
        "name": "run_command",
        "kind": "execute"
    });
    match &translate(&started)[..] {
        [Event::ToolStarted { id, title, .. }] => {
            assert_eq!(id, "t1");
            assert_eq!(title, "Echo hi");
        }
        other => panic!("want a tool start, got {other:?}"),
    }

    let finished = json!({
        "sessionUpdate": "tool_call_update",
        "toolCallId": "t1",
        "status": "completed",
        "content": [{ "type": "content", "content": { "type": "text", "text": "hi" } }]
    });
    match &translate(&finished)[..] {
        [Event::ToolFinished { status, output, .. }] => {
            assert_eq!(status, "completed");
            assert_eq!(output, "hi");
        }
        other => panic!("want a tool finish, got {other:?}"),
    }
}

#[test]
fn tool_output_falls_back_to_raw_output() {
    let update = json!({
        "sessionUpdate": "tool_call_update",
        "toolCallId": "t2",
        "status": "completed",
        "rawOutput": "from raw"
    });
    match &translate(&update)[..] {
        [Event::ToolFinished { output, .. }] => assert_eq!(output, "from raw"),
        other => panic!("want a tool finish, got {other:?}"),
    }
}

#[test]
fn unknown_updates_are_dropped() {
    let update = json!({ "sessionUpdate": "something_new" });
    assert!(translate(&update).is_empty());
}
