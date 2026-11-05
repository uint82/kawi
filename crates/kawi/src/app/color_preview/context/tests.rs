use super::value::ranges as value_ranges;
use super::{preview_ranges, MarkupState};
use crate::app::color_preview::mode::PreviewMode;

fn contains(ranges: &[std::ops::Range<usize>], index: usize) -> bool {
    ranges
        .iter()
        .any(|range| range.start <= index && index < range.end)
}

#[test]
fn keeps_values_and_drops_selectors_and_strings() {
    let line = "a:hover .red { color: red; content: \"red\"; }";
    let ranges = value_ranges(line, false);
    let selector = line.find(".red").expect("selector") + 1;
    let value = line.find("color: red").expect("value") + 7;
    let string = line.find("\"red\"").expect("string") + 1;
    assert!(!contains(&ranges, selector));
    assert!(contains(&ranges, value));
    assert!(!contains(&ranges, string));
}

#[test]
fn keeps_commented_values_out() {
    let line = "color: /* red */ blue;";
    let ranges = value_ranges(line, false);
    let comment = line.find("red").expect("comment");
    let value = line.find("blue").expect("value");
    assert!(!contains(&ranges, comment));
    assert!(contains(&ranges, value));
}

#[test]
fn commits_style_attribute_values_at_the_end() {
    let line = "color: red";
    let ranges = value_ranges(line, true);
    assert!(contains(&ranges, line.find("red").expect("value")));
}

#[test]
fn plain_mode_allows_the_whole_line() {
    let line = "red";
    let mut state = MarkupState::default();
    let ranges = preview_ranges(line, PreviewMode::Plain, &mut state);
    assert!(contains(&ranges, 0));
}
