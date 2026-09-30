use super::*;
use crate::wrap;

fn drawn(ratio: f64, width: usize) -> String {
    bar(ratio, width)
        .iter()
        .map(|s| s.content.as_ref())
        .collect()
}

#[test]
fn bar_always_spans_the_width() {
    for ratio in [0.0, 0.33, 0.5, 0.999, 1.0] {
        assert_eq!(wrap::width(&drawn(ratio, 20)), 20, "{ratio}");
    }
}

#[test]
fn bar_fills_in_eighths() {
    assert_eq!(drawn(0.5, 4), "██──");
    assert_eq!(drawn(0.625, 4), "██▌─");
    assert_eq!(drawn(0.5625, 4), "██▎─");
}

#[test]
fn bar_clamps_out_of_range_ratios() {
    assert_eq!(drawn(-1.0, 3), "───");
    assert_eq!(drawn(7.0, 3), "███");
}
