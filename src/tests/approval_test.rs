use super::*;

fn choice(id: &str, kind: ChoiceKind) -> Choice {
    Choice {
        id: id.into(),
        label: id.into(),
        kind,
    }
}

fn request(id: u64, preview: &[&str]) -> Request<u64> {
    Request {
        title: format!("tool {id}"),
        preview: preview.iter().map(|s| (*s).to_string()).collect(),
        choices: vec![
            choice("once", ChoiceKind::AllowOnce),
            choice("always", ChoiceKind::AllowAlways),
            choice("no", ChoiceKind::RejectOnce),
        ],
        payload: id,
    }
}

#[test]
fn opens_on_the_one_time_allow() {
    let mut a = Approvals::default();
    a.push(request(1, &[]));
    assert_eq!(a.selected, 0);
}

#[test]
fn never_preselects_allow_always() {
    let mut a = Approvals::default();
    let mut r = request(1, &[]);
    r.choices.remove(0);
    r.choices.reverse();
    a.push(r);
    assert_eq!(
        a.current().unwrap().choices[a.selected],
        choice("always", ChoiceKind::AllowAlways)
    );
}

#[test]
fn extra_requests_queue_behind_the_current_one() {
    let mut a = Approvals::default();
    a.push(request(1, &[]));
    a.push(request(2, &[]));
    assert_eq!(a.waiting(), 1);

    let (answered, picked) = a.answer().unwrap();
    assert_eq!((answered.payload, picked.id.as_str()), (1, "once"));
    assert_eq!(a.current().map(|r| r.payload), Some(2));
    assert_eq!(a.waiting(), 0);
}

#[test]
fn reject_picks_the_denying_answer() {
    let mut a = Approvals::default();
    a.push(request(1, &[]));
    let (_, picked) = a.reject().unwrap();
    assert_eq!(picked, choice("no", ChoiceKind::RejectOnce));
    assert!(!a.is_open());
}

#[test]
fn selection_wraps() {
    let mut a = Approvals::default();
    a.push(request(1, &[]));
    a.move_by(-1);
    assert_eq!(a.selected, 2);
    a.move_by(1);
    assert_eq!(a.selected, 0);
}

#[test]
fn preview_is_capped() {
    let rows: Vec<String> = (0..30).map(|i| format!("line {i}")).collect();
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    let mut a = Approvals::default();
    a.push(request(1, &refs));
    let (shown, hidden) = a.preview();
    assert_eq!(
        (shown.len(), hidden),
        (MAX_PREVIEW_ROWS, 30 - MAX_PREVIEW_ROWS)
    );
}

#[test]
fn height_covers_every_row_it_draws() {
    let mut a = Approvals::default();
    a.push(request(1, &["do a thing"]));
    assert_eq!(a.height(), 6);
}

#[test]
fn a_request_with_nothing_to_pick_stays_open() {
    let mut a = Approvals::default();
    let mut r = request(1, &[]);
    r.choices.clear();
    a.push(r);
    assert_eq!(a.answer(), None);
    assert!(a.is_open());
}
