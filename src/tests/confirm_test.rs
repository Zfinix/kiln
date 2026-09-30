use super::*;

fn press(confirm: &mut Confirm, code: KeyCode) {
    confirm.handle_key(KeyEvent::from(code));
}

#[test]
fn confirm_enter_takes_the_safe_default() {
    let mut c = Confirm::new("Delete?");
    press(&mut c, KeyCode::Enter);
    assert_eq!(c.answer(), Some(false));
}

#[test]
fn confirm_moving_then_enter_says_yes() {
    let mut c = Confirm::new("Delete?");
    press(&mut c, KeyCode::Right);
    press(&mut c, KeyCode::Enter);
    assert_eq!(c.answer(), Some(true));
}

#[test]
fn confirm_letters_answer_at_once() {
    let mut c = Confirm::new("Delete?").default_yes();
    press(&mut c, KeyCode::Char('n'));
    assert_eq!(c.answer(), Some(false));
    assert!(c.is_complete());
}

#[test]
fn confirm_esc_means_no() {
    let mut c = Confirm::new("Delete?").default_yes();
    press(&mut c, KeyCode::Esc);
    assert_eq!(c.answer(), Some(false));
}
