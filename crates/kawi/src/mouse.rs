use crossterm::event::{MouseEvent, MouseEventKind};
pub const WHEEL_LINES: isize = 3;
pub const WHEEL_COLUMNS: isize = 6;

pub fn wheel_delta(event: &MouseEvent) -> Option<isize> {
    match event.kind {
        MouseEventKind::ScrollUp => Some(-WHEEL_LINES),
        MouseEventKind::ScrollDown => Some(WHEEL_LINES),
        _ => None,
    }
}

pub fn horizontal_wheel_delta(event: &MouseEvent) -> Option<isize> {
    match event.kind {
        MouseEventKind::ScrollLeft => Some(-WHEEL_COLUMNS),
        MouseEventKind::ScrollRight => Some(WHEEL_COLUMNS),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyModifiers, MouseButton};

    fn event(kind: MouseEventKind) -> MouseEvent {
        MouseEvent {
            kind,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn vertical_wheel_delta_is_directional() {
        assert_eq!(
            wheel_delta(&event(MouseEventKind::ScrollUp)),
            Some(-WHEEL_LINES)
        );
        assert_eq!(
            wheel_delta(&event(MouseEventKind::ScrollDown)),
            Some(WHEEL_LINES)
        );
    }

    #[test]
    fn horizontal_wheel_delta_is_directional() {
        assert_eq!(
            horizontal_wheel_delta(&event(MouseEventKind::ScrollLeft)),
            Some(-WHEEL_COLUMNS)
        );
        assert_eq!(
            horizontal_wheel_delta(&event(MouseEventKind::ScrollRight)),
            Some(WHEEL_COLUMNS)
        );
    }

    #[test]
    fn other_mouse_events_do_not_scroll() {
        assert_eq!(
            wheel_delta(&event(MouseEventKind::Down(MouseButton::Left))),
            None
        );
        assert_eq!(
            horizontal_wheel_delta(&event(MouseEventKind::Down(MouseButton::Left))),
            None
        );
    }
}
