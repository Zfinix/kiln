use super::*;
use crate::wrap;

#[test]
fn lines_rows_share_one_width() {
    let rows = lines("12:05", Style::default());
    let widths: Vec<usize> = rows.iter().map(|l| wrap::width(&l.to_string())).collect();
    assert_eq!(widths, vec![width("12:05"); HEIGHT]);
}

#[test]
fn lines_draw_each_digit_distinctly() {
    let drawn: Vec<String> = (0..10)
        .map(|d| {
            lines(&d.to_string(), Style::default())
                .iter()
                .map(|l| l.to_string())
                .collect()
        })
        .collect();
    let unique: std::collections::HashSet<&String> = drawn.iter().collect();
    assert_eq!(unique.len(), 10);
}

#[test]
fn width_counts_the_gap_between_glyphs() {
    assert_eq!(width("88"), 3 + 1 + 3);
    assert_eq!(width(""), 0);
}
