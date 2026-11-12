use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_html_tags_attributes_and_entities() {
    let text = "<!doctype html>\n<!-- shell -->\n<div class=\"box\" data-id=page hidden>\n  <span title='tip'>&amp;</span>\n</div>\n";
    let spans = highlight(LanguageId::Html, text, 0, 5);
    assert_eq!(name_of(&spans, text, 0, "doctype"), Some("constant"));
    assert_eq!(name_at(&spans, 0, 0), Some("tag.delimiter"));
    assert_eq!(name_of(&spans, text, 1, "shell"), Some("comment"));
    assert_eq!(name_of(&spans, text, 2, "div"), Some("tag"));
    assert_eq!(name_of(&spans, text, 2, "class"), Some("tag.attribute"));
    assert_eq!(name_of(&spans, text, 2, "box"), Some("string"));
    assert_eq!(name_of(&spans, text, 2, "data-id"), Some("tag.attribute"));
    assert_eq!(name_of(&spans, text, 2, "page"), Some("string"));
    assert_eq!(name_of(&spans, text, 3, "span"), Some("tag"));
    assert_eq!(name_of(&spans, text, 3, "title"), Some("tag.attribute"));
    assert_eq!(name_of(&spans, text, 3, "tip"), Some("string"));
    assert_eq!(name_of(&spans, text, 3, "&amp;"), Some("character.special"));
    let open = text.lines().nth(2).expect("row").find('<').expect("<");
    assert_eq!(name_at(&spans, 2, open), Some("tag.delimiter"));
    let close = text.lines().nth(3).expect("row").find("</").expect("</");
    assert_eq!(name_at(&spans, 3, close), Some("tag.delimiter"));
    let equals = text.lines().nth(2).expect("row").find('=').expect("=");
    assert_eq!(name_at(&spans, 2, equals), Some("operator"));
}

#[test]
fn highlights_html_text_semantics() {
    let text = "<title>Demo</title>\n<h1>Welcome</h1>\n<h2>Intro</h2>\n<p>Hello <strong>bold</strong> and <em>soft</em>, <code>sample</code>, <s>stale</s>, <u>under</u>.</p>\n<p><b>heavy</b> <i>slant</i> <del>gone</del> <kbd>key</kbd></p>\n<a href=\"/home\">Home</a>\n";
    let spans = highlight(LanguageId::Html, text, 0, 6);
    assert_eq!(name_of(&spans, text, 0, "Demo"), Some("markup.heading"));
    assert_eq!(
        name_of(&spans, text, 1, "Welcome"),
        Some("markup.heading.1")
    );
    assert_eq!(name_of(&spans, text, 2, "Intro"), Some("markup.heading.2"));
    assert_eq!(name_of(&spans, text, 3, "bold"), Some("markup.strong"));
    assert_eq!(name_of(&spans, text, 3, "soft"), Some("markup.italic"));
    assert_eq!(name_of(&spans, text, 3, "sample"), Some("markup.raw"));
    assert_eq!(
        name_of(&spans, text, 3, "stale"),
        Some("markup.strikethrough")
    );
    assert_eq!(name_of(&spans, text, 3, "under"), Some("markup.underline"));
    assert_eq!(name_of(&spans, text, 4, "heavy"), Some("markup.strong"));
    assert_eq!(name_of(&spans, text, 4, "slant"), Some("markup.italic"));
    assert_eq!(
        name_of(&spans, text, 4, "gone"),
        Some("markup.strikethrough")
    );
    assert_eq!(name_of(&spans, text, 4, "key"), Some("markup.raw"));
    assert_eq!(name_of(&spans, text, 5, "Home"), Some("markup.link.label"));
}

#[test]
fn highlights_html_urls_in_quoted_and_unquoted_attributes() {
    let text = "<a href=\"/home\" title=\"x\">Home</a>\n<img src=logo.png alt=logo />\n<form action=\"/send\"></form>\n";
    let spans = highlight(LanguageId::Html, text, 0, 3);
    assert_eq!(name_of(&spans, text, 0, "href"), Some("tag.attribute"));
    assert_eq!(
        name_of(&spans, text, 0, "/home"),
        Some("string.special.url")
    );
    assert_eq!(name_of(&spans, text, 0, "\"x\""), Some("string"));
    assert_eq!(name_of(&spans, text, 1, "src"), Some("tag.attribute"));
    assert_eq!(
        name_of(&spans, text, 1, "logo.png"),
        Some("string.special.url")
    );
    let alt = text.lines().nth(1).expect("row").find("alt").expect("alt");
    let value = text[alt..].find('=').expect("=") + 1;
    assert_eq!(name_at(&spans, 1, alt + value), Some("string"));
    assert_eq!(name_of(&spans, text, 2, "action"), Some("tag.attribute"));
    let send = text
        .lines()
        .nth(2)
        .expect("row")
        .find("/send")
        .expect("/send");
    assert_eq!(name_at(&spans, 2, send), Some("string"));
}

#[test]
fn highlights_html_self_closing_tags_and_errors() {
    let text = "<br />\n<img src=\"logo.png\" />\n<div class=\"box\"></span>\n";
    let spans = highlight(LanguageId::Html, text, 0, 3);
    assert_eq!(name_of(&spans, text, 0, "br"), Some("tag"));
    assert_eq!(name_of(&spans, text, 1, "src"), Some("tag.attribute"));
    assert_eq!(name_of(&spans, text, 2, "span"), Some("tag.error"));
    let self_close = text.lines().next().expect("row").find("/>").expect("/>");
    assert_eq!(name_at(&spans, 0, self_close), Some("tag.delimiter"));
}

#[test]
fn leaves_plain_html_text_unstyled() {
    let text = "<p>Plain text</p>\n<pre>Literal text</pre>\n";
    let spans = highlight(LanguageId::Html, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "Plain"), None);
    assert_eq!(name_of(&spans, text, 1, "Literal"), None);
}
