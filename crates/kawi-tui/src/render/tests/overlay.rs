use super::*;
use crate::render::overlay::{overlay_background, scrollbar_thumb};

#[test]
fn code_lines_use_the_code_background() {
    let theme = Theme::named("default").expect("default theme");
    let code = crate::markdown::MarkdownLine::code(Vec::new());
    let text = crate::markdown::MarkdownLine::blank();
    assert_eq!(overlay_background(&code, &theme), theme.code_background);
    assert_eq!(overlay_background(&text, &theme), theme.float_background);
}

#[test]
fn scrollbar_thumb_tracks_the_visible_range() {
    assert_eq!(scrollbar_thumb(0, 10, 10), None);
    assert_eq!(scrollbar_thumb(0, 100, 10), Some(0..1));
    assert_eq!(scrollbar_thumb(45, 100, 10), Some(4..5));
    assert_eq!(scrollbar_thumb(90, 100, 10), Some(9..10));
    assert_eq!(scrollbar_thumb(90, 11, 10), Some(1..10));
}
