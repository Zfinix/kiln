use super::*;

#[test]
fn line_scales_to_the_peak() {
    assert_eq!(line(&[0.0, 4.0, 8.0], 3, Style::default()).content, " ▄█");
}

#[test]
fn line_keeps_only_the_newest_values() {
    assert_eq!(line(&[8.0, 0.0, 8.0], 2, Style::default()).content, " █");
}

#[test]
fn line_pads_a_short_history_on_the_left() {
    assert_eq!(line(&[1.0], 4, Style::default()).content, "   █");
}

#[test]
fn line_of_all_zeros_is_blank() {
    assert_eq!(line(&[0.0, 0.0], 2, Style::default()).content, "  ");
}
