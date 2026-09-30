use super::*;

fn typed(input: &mut Input, text: &str) {
    text.chars()
        .for_each(|c| input.handle_key(KeyEvent::from(KeyCode::Char(c))));
}

fn key(input: &mut Input, code: KeyCode) {
    input.handle_key(KeyEvent::from(code));
}

fn ctrl(input: &mut Input, c: char) {
    input.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
}

#[test]
fn input_limit_refuses_extra_characters() {
    let mut i = Input::new("> ").limit(3);
    typed(&mut i, "abcdef");
    assert_eq!(i.text(), "abc");
}

#[test]
fn input_edits_at_the_caret() {
    let mut i = Input::new("> ");
    typed(&mut i, "héllo");
    key(&mut i, KeyCode::Left);
    key(&mut i, KeyCode::Left);
    key(&mut i, KeyCode::Backspace);
    typed(&mut i, "L");
    assert_eq!(i.text(), "héLlo");
}

#[test]
fn input_ctrl_w_deletes_the_previous_word() {
    let mut i = Input::new("> ").value("fix the thing");
    ctrl(&mut i, 'w');
    assert_eq!(i.text(), "fix the ");
}

#[test]
fn input_ctrl_u_deletes_to_the_start() {
    let mut i = Input::new("> ").value("fix the thing");
    key(&mut i, KeyCode::Left);
    ctrl(&mut i, 'u');
    assert_eq!(i.text(), "g");
}

#[test]
fn input_enter_submits_and_esc_cancels() {
    let mut i = Input::new("> ");
    key(&mut i, KeyCode::Enter);
    assert_eq!(i.outcome(), Some(Outcome::Submitted));
    let mut j = Input::new("> ");
    key(&mut j, KeyCode::Esc);
    assert_eq!(j.outcome(), Some(Outcome::Cancelled));
}

#[test]
fn input_paste_drops_newlines_and_respects_the_limit() {
    let mut i = Input::new("> ").limit(5);
    i.handle_paste("ab\ncdefg".into());
    assert_eq!(i.text(), "abcde");
}

#[test]
fn input_caret_stays_in_view_when_the_value_overflows() {
    let i = Input::new("> ").value("a".repeat(50));
    let area = Rect::new(0, 0, 20, 1);
    let (col, _) = i.cursor_pos(area).unwrap();
    assert!(col < area.width, "{col}");
}
