use super::*;

fn menu() -> Menu {
    let mut m = Menu::default();
    m.set_commands(
        ["review", "commit", "refactor"]
            .map(|name| Command::new(name, format!("does {name}")))
            .into(),
    );
    m
}

#[test]
fn filters_by_prefix() {
    let mut m = menu();
    m.sync("/re");
    assert!(m.is_open());
    assert_eq!(m.picked(), Some(&Command::new("review", "does review")));
}

#[test]
fn closes_once_the_command_takes_arguments() {
    let mut m = menu();
    m.sync("/review now");
    assert!(!m.is_open());
}

#[test]
fn ignores_plain_text() {
    let mut m = menu();
    m.sync("hello");
    assert!(!m.is_open());
}

#[test]
fn wraps_around() {
    let mut m = menu();
    m.sync("/");
    m.move_by(-1);
    assert_eq!(m.picked().map(|c| c.name.as_str()), Some("refactor"));
}

#[test]
fn a_long_list_shows_a_window_and_counts_the_rest() {
    let mut m = Menu::default();
    m.set_commands((0..10).map(|i| Command::new(format!("c{i}"), "")).collect());
    m.sync("/");
    let (rows, hidden) = m.window();
    assert_eq!((rows.len(), hidden), (ROWS, 10 - ROWS));
    assert_eq!(m.height(), ROWS as u16 + 1);
}
