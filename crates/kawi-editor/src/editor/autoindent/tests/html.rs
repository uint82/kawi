use super::*;

#[test]
fn html_opening_tag_indents() {
    let mut e = html_editor("<div>");
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[1], "    ");
}

#[test]
fn html_nested_tag_indents() {
    let mut e = html_editor("<div>\n    <p>");
    append_last_line_end(&mut e);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[2], "        ");
}

#[test]
fn html_void_tag_does_not_indent() {
    let mut e = html_editor("<img src=\"logo.png\">");
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[1], "");
}

#[test]
fn html_script_comparison_does_not_indent() {
    let mut e = html_editor("<script>\n    if (a<b && c>d) return;");
    append_last_line_end(&mut e);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[2], "    ");
}

#[test]
fn html_comment_keeps_indent() {
    let mut e = html_editor("<div>\n    <!-- note");
    append_last_line_end(&mut e);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[2], "    ");
}

#[test]
fn html_closing_tag_dedents_to_matching_opener() {
    let mut e = html_editor("<div>\n    <p>");
    e.apply(Action::JumpTo(1, 7));
    e.apply(Action::OpenLineBelow);
    assert_eq!(e.text_lines()[2], "        ");
    for ch in "</p>".chars() {
        e.apply(Action::InsertChar(ch));
    }
    assert_eq!(e.text_lines()[2], "    </p>");
    assert_eq!((e.row(), e.col()), (2, 8));
}

#[test]
fn html_closing_tag_dedent_is_one_undo_step() {
    let mut e = html_editor("<div>\n    <p>\n        ");
    e.apply(Action::JumpTo(2, 0));
    e.apply(Action::AppendLineEnd);
    for ch in "</p>".chars() {
        e.apply(Action::InsertChar(ch));
    }
    e.apply(Action::ExitInsert);
    e.apply(Action::Undo);
    assert_eq!(e.text_lines()[2], "        ");
}

#[test]
fn html_attribute_closer_dedents_to_tag_start() {
    let mut e = html_editor("<div\n    class=\"a\"");
    e.apply(Action::JumpTo(1, 15));
    e.apply(Action::OpenLineBelow);
    assert_eq!(e.text_lines()[2], "    ");
    e.apply(Action::InsertChar('>'));
    assert_eq!(e.text_lines()[2], ">");
    assert_eq!((e.row(), e.col()), (2, 1));
}

#[test]
fn html_multiline_tag_indents_content_after_attribute_closer() {
    let mut e = html_editor("<div\n    class=\"a\"");
    e.apply(Action::JumpTo(1, 15));
    e.apply(Action::OpenLineBelow);
    e.apply(Action::InsertChar('>'));
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[3], "    ");
}
