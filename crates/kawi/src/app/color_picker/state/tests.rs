use super::*;

fn document() -> DocumentColor {
    DocumentColor {
        start: CursorPosition { line: 0, column: 7 },
        end: CursorPosition {
            line: 0,
            column: 14,
        },
        red: 204,
        green: 34,
        blue: 51,
        alpha: 255,
    }
}

fn picker() -> ColorPicker {
    ColorPicker::new(&document(), PathBuf::from("/tmp/color.css"), 3)
}

#[test]
fn starts_from_the_document_color() {
    let picker = picker();
    assert_eq!(picker.color(), Rgba::new(204, 34, 51, 255));
    assert_eq!(picker.field(), Field::Hue);
    assert_eq!(picker.mode(), Mode::Adjust);
    assert_eq!(picker.revision(), 3);
    assert_eq!(picker.range(), (document().start, document().end));
}

#[test]
fn adjusts_fields_and_wraps_hue() {
    let mut picker = picker();
    picker.adjust(false);
    assert_eq!(picker.color().red, 204);
    for _ in 0..4 {
        picker.adjust(true);
    }
    let hue = Field::Hue.value(picker.hsla);
    assert!((0.0..360.0).contains(&hue));
    picker.select_field(true);
    assert_eq!(picker.field(), Field::Saturation);
    picker.adjust(false);
    assert!(Field::Saturation.value(picker.hsla) > 0.0);
}

#[test]
fn clamps_percent_fields() {
    let mut picker = picker();
    picker.select_field(true);
    for _ in 0..30 {
        picker.decrease(true);
    }
    assert_eq!(picker.hsla.saturation, 0.0);
    for _ in 0..30 {
        picker.adjust(true);
    }
    assert_eq!(picker.hsla.saturation, 100.0);
}

#[test]
fn applies_matching_presentations() {
    let mut picker = picker();
    picker.begin_request(9);
    let event = ColorPresentationEvent {
        request: 9,
        path: picker.path().clone(),
        presentations: vec![ColorPresentationEntry {
            label: "#cc2233".to_string(),
            edit: kawi_lsp::TextChange {
                start: 7,
                end: 14,
                text: "#cc2233".to_string(),
            },
            additional: Vec::new(),
        }],
    };
    assert!(picker.apply_event(event));
    assert_eq!(picker.mode(), Mode::Present);
    assert_eq!(picker.selected_presentation().unwrap().label, "#cc2233");
}

#[test]
fn empty_presentations_return_to_adjust() {
    let mut picker = picker();
    picker.begin_request(9);
    let event = ColorPresentationEvent {
        request: 9,
        path: picker.path().clone(),
        presentations: Vec::new(),
    };
    assert!(picker.apply_event(event));
    assert_eq!(picker.mode(), Mode::Adjust);
    assert_eq!(picker.message(), Some("No color formats available"));
}

#[test]
fn failures_return_to_adjust() {
    let mut picker = picker();
    picker.begin_request(9);
    assert!(picker.fail_request(9, "request failed"));
    assert_eq!(picker.mode(), Mode::Adjust);
    assert_eq!(picker.message(), Some("request failed"));
}

#[test]
fn rejects_foreign_presentations() {
    let mut picker = picker();
    picker.begin_request(9);
    let event = ColorPresentationEvent {
        request: 8,
        path: picker.path().clone(),
        presentations: Vec::new(),
    };
    assert!(!picker.apply_event(event));
}

#[test]
fn detects_positions_inside_and_outside_ranges() {
    let color = document();
    assert!(contains_position(
        &color,
        CursorPosition { line: 0, column: 7 }
    ));
    assert!(contains_position(
        &color,
        CursorPosition {
            line: 0,
            column: 13
        }
    ));
    assert!(!contains_position(
        &color,
        CursorPosition {
            line: 0,
            column: 14
        }
    ));
    assert!(!contains_position(
        &color,
        CursorPosition { line: 1, column: 7 }
    ));
}
