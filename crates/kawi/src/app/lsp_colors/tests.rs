use super::*;
use kawi_lsp::CursorPosition;

fn color() -> DocumentColor {
    DocumentColor {
        start: CursorPosition { line: 0, column: 0 },
        end: CursorPosition { line: 0, column: 4 },
        red: 255,
        green: 0,
        blue: 0,
        alpha: 255,
    }
}

fn event(request: u64, path: &Path) -> DocumentColorsEvent {
    DocumentColorsEvent {
        request,
        path: path.to_path_buf(),
        colors: vec![color()],
    }
}

#[test]
fn requests_after_the_delay() {
    let mut state = DocumentColorState::default();
    let now = Instant::now();
    let path = PathBuf::from("/tmp/colors.css");
    state.document_changed(None, Some(path.clone()), 1, true, now);
    assert_eq!(state.wait_timeout(now), Some(COLOR_REQUEST_DELAY));
    assert!(!state.request_if_due(now, true, |_| 7));
    assert!(
        state.request_if_due(now + COLOR_REQUEST_DELAY, true, |requested| {
            assert_eq!(requested, path);
            7
        })
    );
    assert_eq!(state.request, Some(7));
    assert_eq!(state.wait_timeout(now), None);
}

#[test]
fn skips_requests_without_support() {
    let mut state = DocumentColorState::default();
    let now = Instant::now();
    state.document_changed(None, Some(PathBuf::from("/tmp/colors.css")), 1, false, now);
    assert_eq!(state.wait_timeout(now), None);
    assert!(!state.request_if_due(now, false, |_| 7));
    assert_eq!(state.request, None);
}

#[test]
fn stores_current_document_colors() {
    let mut state = DocumentColorState::default();
    let now = Instant::now();
    let path = PathBuf::from("/tmp/colors.css");
    state.document_changed(None, Some(path.clone()), 1, true, now);
    state.request_if_due(now + COLOR_REQUEST_DELAY, true, |_| 7);
    assert!(state.apply_event(event(7, &path), Some(&path), Some(1)));
    assert_eq!(state.colors(&path, 1).expect("colors").len(), 1);
    assert!(state.colors(&path, 2).is_none());
    assert!(state.colors(Path::new("/tmp/other.css"), 1).is_none());
}

#[test]
fn ignores_stale_responses() {
    let mut state = DocumentColorState::default();
    let now = Instant::now();
    let path = PathBuf::from("/tmp/colors.css");
    state.document_changed(None, Some(path.clone()), 1, true, now);
    state.request_if_due(now + COLOR_REQUEST_DELAY, true, |_| 7);
    assert!(!state.apply_event(event(8, &path), Some(&path), Some(1)));
    assert!(!state.apply_event(event(7, &path), Some(&path), Some(2)));
    assert!(state.colors(&path, 1).is_none());
}

#[test]
fn disabling_clears_colors_and_reschedules_on_enable() {
    let mut state = DocumentColorState::default();
    let now = Instant::now();
    let path = PathBuf::from("/tmp/colors.css");
    state.document_changed(None, Some(path.clone()), 1, true, now);
    state.request_if_due(now + COLOR_REQUEST_DELAY, true, |_| 7);
    state.apply_event(event(7, &path), Some(&path), Some(1));
    assert!(state.set_enabled(false, None, true, now));
    assert!(state.colors(&path, 1).is_none());
    assert_eq!(state.wait_timeout(now), None);
    assert!(state.set_enabled(true, None, true, now));
    assert_eq!(state.wait_timeout(now), Some(COLOR_REQUEST_DELAY));
}
