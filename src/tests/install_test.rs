use std::time::Duration;

use super::*;

fn drawn(line: &Line<'_>) -> String {
    line.to_string()
}

#[test]
fn durations_read_like_uv() {
    let said: Vec<String> = [
        Duration::from_millis(174),
        Duration::from_millis(2_317),
        Duration::from_secs(64),
    ]
    .map(duration)
    .into();
    assert_eq!(said, ["174ms", "2.31s", "1m 04s"]);
}

#[test]
fn summary_names_the_verb_the_count_and_the_time() {
    let line = summary("Installed", "90 packages", Duration::from_millis(174));
    assert_eq!(drawn(&line), "Installed 90 packages in 174ms");
}

#[test]
fn changes_mark_what_happened() {
    let said: Vec<String> = [Change::Added, Change::Removed, Change::Reinstalled]
        .map(|kind| drawn(&change(kind, "harbor", "0.23.0")))
        .into();
    assert_eq!(
        said,
        [
            " + harbor==0.23.0",
            " - harbor==0.23.0",
            " ~ harbor==0.23.0"
        ]
    );
}

#[test]
fn header_shows_the_count_after_the_message() {
    let line = header(5, "Preparing packages...", Some((62, 63)), 80);
    assert_eq!(drawn(&line), "⠴ Preparing packages... (62/63)");
}

#[test]
fn bar_fills_in_proportion() {
    let line = bar("litellm", 8, 15 << 20, 30 << 20, 80);
    let spans: Vec<&str> = line.spans.iter().map(|s| s.content.as_ref()).collect();
    let half = "-".repeat(BAR_WIDTH / 2);
    assert_eq!(spans, ["litellm  ", &half, &half, "  15.00 MiB/30.00 MiB"]);
}

#[test]
fn bar_never_runs_past_the_width() {
    for width in [32, 40, 60, 120] {
        let line = bar("a-very-long-package-name", 24, 1, 2, width);
        assert!(wrap::width(&drawn(&line)) <= width, "{width}");
    }
}
