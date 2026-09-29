use super::*;

#[test]
fn clip_row_keeps_short_text_and_cuts_long_text_at_the_column_budget() {
    assert_eq!(clip_row("short", 20), "short");
    let long = clip_row("a".repeat(50).as_str(), 20);
    assert!(long.ends_with('…'), "{long}");
    assert!(wrap::width(&long) <= 20, "{long}");
}

#[test]
fn clip_row_measures_wide_glyphs_in_columns_not_chars() {
    let cjk = "寫論文學術論文引導我寫論文審查意見評估回覆論文審查";
    let clipped = clip_row(cjk, 20);
    assert!(clipped.ends_with('…'), "{clipped}");
    assert!(wrap::width(&clipped) <= 20, "{clipped}");
}

#[test]
fn clip_row_flattens_newlines_into_one_row() {
    assert_eq!(clip_row("one\ntwo", 20), "one two");
}

#[test]
fn counts_read_as_a_person_would_say_them() {
    let said: Vec<String> = [999, 1_000, 1_250, 3_400_000].map(human_count).into();
    assert_eq!(said, ["999", "1k", "1.2k", "3.4M"]);
}

#[test]
fn a_long_run_reads_as_minutes_not_a_seconds_count() {
    assert_eq!(elapsed(209), "3m 29s");
    assert_eq!(elapsed(60), "1m 0s");
    assert_eq!(elapsed(59), "59s");
}

#[test]
fn a_long_list_is_capped_and_its_tail_split_off() {
    let names: Vec<String> = (0..10).map(|i| format!("n{i}")).collect();
    let value = listed(names.iter().map(String::as_str));
    assert_eq!(
        split_more(&value),
        ("n0, n1, n2, n3, n4, n5, n6, n7", Some("… +2 more"))
    );
}

#[test]
fn ansi_output_carries_weight_and_colour_and_resets_after_each_span() {
    use ratatui::style::{Color, Style};
    use ratatui::text::Span;
    let line = Line::from(vec![
        Span::raw("plain "),
        Span::styled("bold", Style::default().bold().fg(Color::Rgb(1, 2, 3))),
        Span::styled(
            " dim",
            Style::default().fg(Color::Red).bg(Color::Indexed(236)),
        ),
    ]);
    assert_eq!(
        to_ansi(&[line]),
        "plain \x1b[1;38;2;1;2;3mbold\x1b[0m\x1b[31;48;5;236m dim\x1b[0m\n"
    );
}

#[test]
fn ansi_input_becomes_styled_spans_on_top_of_the_base_style() {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::Span;
    let base = Style::default().fg(Color::Rgb(9, 9, 9));
    let row = "\x1b]0;title\x07ok \x1b[1;32mpassed\x1b[0m \x1b[38;2;1;2;3mrgb\x1b[39m done\x1b[K";
    assert_eq!(
        from_ansi(row, base),
        vec![
            Span::styled("ok ", base),
            Span::styled("passed", base.fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(" ", base),
            Span::styled("rgb", base.fg(Color::Rgb(1, 2, 3))),
            Span::styled(" done", base),
        ]
    );
}

#[test]
fn ansi_round_trips_through_to_ansi() {
    use ratatui::style::Style;
    let row = "\x1b[31mred\x1b[0m \x1b[1;38;2;1;2;3mbold rgb\x1b[0m";
    let line = Line::from(from_ansi(row, Style::default()));
    assert_eq!(to_ansi(&[line]), format!("{row}\n"));
}
