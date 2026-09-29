//! Small string helpers for rows that must fit a width: clipping by display
//! column, compact counts and durations, and capped name lists.

use std::path::Path;

use crate::wrap;

pub const LIST_MAX: usize = 8;

const MORE: &str = "… +";

/// `path` with the home directory folded to `~`.
pub fn short_path(path: &Path) -> String {
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
        && let Ok(rel) = path.strip_prefix(&home)
    {
        return format!("~/{}", rel.display());
    }
    path.display().to_string()
}

/// A count as a reader states it: `999`, `1.2k`, `3.4M`.
pub fn human_count(n: u64) -> String {
    match n {
        0..=999 => n.to_string(),
        1_000..=999_999 => trim_zero(n as f64 / 1_000.0, 'k'),
        _ => trim_zero(n as f64 / 1_000_000.0, 'M'),
    }
}

fn trim_zero(value: f64, unit: char) -> String {
    let text = format!("{value:.1}");
    let text = text.strip_suffix(".0").unwrap_or(&text);
    format!("{text}{unit}")
}

/// A running time: seconds under a minute, `3m 29s` past one.
pub fn elapsed(secs: u64) -> String {
    match secs >= 60 {
        true => format!("{}m {}s", secs / 60, secs % 60),
        false => format!("{secs}s"),
    }
}

/// `1 file`, `3 files`.
pub fn count_of(n: usize, noun: &str) -> String {
    format!("{n} {noun}{}", if n == 1 { "" } else { "s" })
}

/// Join names with commas, keeping the first [`LIST_MAX`] and counting the rest.
pub fn listed<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let all: Vec<&str> = names.collect();
    if all.len() <= LIST_MAX {
        return all.join(", ");
    }
    format!(
        "{} {MORE}{} more",
        all[..LIST_MAX].join(", "),
        all.len() - LIST_MAX
    )
}

/// Split a [`listed`] value into its names and the `… +N more` tail.
pub fn split_more(value: &str) -> (&str, Option<&str>) {
    match value.find(MORE) {
        Some(at) => (value[..at].trim_end(), Some(&value[at..])),
        None => (value, None),
    }
}

/// Clip to `max` display columns with an ellipsis. Measured in columns, not
/// chars, so wide CJK text still lands on one row.
pub fn clip_row(text: &str, max: usize) -> String {
    let flat = text.replace('\n', " ");
    if wrap::width(&flat) <= max {
        return flat;
    }
    let head = wrap::rows(&flat, max.saturating_sub(1).max(1))
        .first()
        .map(|r| flat[r.clone()].to_string())
        .unwrap_or_default();
    format!("{}…", head.trim_end())
}

#[cfg(test)]
#[path = "tests/text_test.rs"]
mod tests;
