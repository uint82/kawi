use kawi_plugin::DecorationStyle;

use super::*;

fn segment(text: &str, priority: u32) -> StatuslineSegment {
    StatuslineSegment {
        text: text.to_string(),
        style: DecorationStyle::Info,
        priority,
    }
}

#[test]
fn replaces_the_plugin_segments() {
    let mut statusline = PluginStatusline::default();
    statusline.set("alpha", &[segment("one", 1)]);
    statusline.set("alpha", &[segment("two", 1)]);
    let segments = statusline.segments();
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].text, "two");
}

#[test]
fn orders_by_priority_then_plugin_name() {
    let mut statusline = PluginStatusline::default();
    statusline.set("beta", &[segment("low", 1)]);
    statusline.set("alpha", &[segment("high", 5), segment("low", 1)]);
    let texts = statusline
        .segments()
        .into_iter()
        .map(|segment| segment.text)
        .collect::<Vec<_>>();
    assert_eq!(texts, vec!["high", "low", "low"]);
}

#[test]
fn sanitizes_and_drops_empty_segments() {
    let mut statusline = PluginStatusline::default();
    statusline.set("alpha", &[segment("bad\u{1b}[31m", 1), segment("", 1)]);
    let segments = statusline.segments();
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].text, "bad");
}

#[test]
fn removing_a_plugin_drops_its_segments() {
    let mut statusline = PluginStatusline::default();
    statusline.set("alpha", &[segment("one", 1)]);
    statusline.remove_plugin("alpha");
    assert!(statusline.segments().is_empty());
}
