use super::*;
use serde_json::json;

fn span(text: &str) -> OverlaySpan {
    OverlaySpan {
        text: text.to_string(),
        style: None,
    }
}

fn open_params(lines: Vec<OverlayLine>) -> OverlayOpenParams {
    OverlayOpenParams {
        title: "Menu".to_string(),
        status: None,
        anchor: OverlayAnchor::Center,
        width: None,
        height: None,
        focus: false,
        selected: None,
        scroll: None,
        lines,
    }
}

#[test]
fn open_defaults_are_stable() {
    let params: OverlayOpenParams = serde_json::from_value(json!({
        "title": "Menu",
        "lines": [],
    }))
    .unwrap();
    assert_eq!(params.anchor, OverlayAnchor::Cursor);
    assert_eq!(params.width, None);
    assert!(!params.focus);
}

#[test]
fn rejects_out_of_range_dimensions() {
    let mut params = open_params(Vec::new());
    params.width = Some(MAX_OVERLAY_WIDTH + 1);
    assert!(validate_overlay_open(&params).is_err());
    params.width = None;
    params.height = Some(MIN_OVERLAY_HEIGHT - 1);
    assert!(validate_overlay_open(&params).is_err());
}

#[test]
fn accepts_dimension_boundaries() {
    let mut params = open_params(Vec::new());
    params.width = Some(MIN_OVERLAY_WIDTH);
    params.height = Some(MAX_OVERLAY_HEIGHT);
    assert!(validate_overlay_open(&params).is_ok());
}

#[test]
fn rejects_an_oversized_span() {
    let params = open_params(vec![OverlayLine {
        spans: vec![span(&"x".repeat(MAX_OVERLAY_LINE_BYTES + 1))],
    }]);
    assert!(validate_overlay_open(&params).is_err());
}

#[test]
fn rejects_a_line_that_is_oversized_across_spans() {
    let half = "x".repeat(MAX_OVERLAY_LINE_BYTES / 2 + 1);
    let params = open_params(vec![OverlayLine {
        spans: vec![span(&half), span(&half)],
    }]);
    assert!(validate_overlay_open(&params).is_err());
}

#[test]
fn rejects_content_over_the_total_limit() {
    let line = OverlayLine {
        spans: vec![span(&"x".repeat(MAX_OVERLAY_LINE_BYTES))],
    };
    let params = open_params(vec![line; MAX_OVERLAY_BYTES / MAX_OVERLAY_LINE_BYTES + 1]);
    assert!(validate_overlay_open(&params).is_err());
}

#[test]
fn rejects_an_oversized_title_or_status() {
    let mut params = open_params(Vec::new());
    params.title = "x".repeat(MAX_OVERLAY_TITLE_BYTES + 1);
    assert!(validate_overlay_open(&params).is_err());
    params.title = "Menu".to_string();
    params.status = Some("x".repeat(MAX_OVERLAY_STATUS_BYTES + 1));
    assert!(validate_overlay_open(&params).is_err());
}

#[test]
fn update_validates_only_supplied_fields() {
    let params = OverlayUpdateParams {
        overlay: 1,
        title: Some("Updated".to_string()),
        status: None,
        anchor: None,
        width: None,
        height: None,
        focus: None,
        selected: None,
        scroll: None,
        lines: None,
    };
    assert!(validate_overlay_update(&params).is_ok());
}

#[test]
fn events_round_trip() {
    let event = OverlayEvent::Key(OverlayKeyEvent {
        key: Key::Character('j'),
        modifiers: vec![KeyModifier::Control],
        text: Some("j".to_string()),
    });
    let value = serde_json::to_value(&event).unwrap();
    assert_eq!(value["kind"], "key");
    assert_eq!(
        serde_json::from_value::<OverlayEvent>(value).unwrap(),
        event
    );
}
