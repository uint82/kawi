use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use kawi_tui::viewport::Viewport;

pub const WHEEL_LINES: isize = 3;

pub fn press_position(event: &MouseEvent, viewport: &Viewport) -> Option<(usize, usize)> {
    if !is_left_press(event.kind) {
        return None;
    }
    to_buffer_position(event.column, event.row, viewport)
}

pub fn drag_position(event: &MouseEvent, viewport: &Viewport) -> Option<(usize, usize)> {
    if !is_left_drag(event.kind) {
        return None;
    }
    to_buffer_position(event.column, event.row, viewport)
}

pub fn wheel_delta(event: &MouseEvent) -> Option<isize> {
    match event.kind {
        MouseEventKind::ScrollUp => Some(-WHEEL_LINES),
        MouseEventKind::ScrollDown => Some(WHEEL_LINES),
        _ => None,
    }
}

fn is_left_press(kind: MouseEventKind) -> bool {
    matches!(kind, MouseEventKind::Down(MouseButton::Left))
}

fn is_left_drag(kind: MouseEventKind) -> bool {
    matches!(kind, MouseEventKind::Drag(MouseButton::Left))
}

fn to_buffer_position(column: u16, screen_row: u16, viewport: &Viewport) -> Option<(usize, usize)> {
    let row = usize::from(screen_row).saturating_add(viewport.top());
    let col = usize::from(column);
    if usize::from(screen_row) >= viewport.height() {
        return None;
    }
    Some((row, col))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: crossterm::event::KeyModifiers::NONE,
        }
    }

    fn viewport() -> Viewport {
        Viewport::new(10, 80)
    }

    fn drag(column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column,
            row,
            modifiers: crossterm::event::KeyModifiers::NONE,
        }
    }

    #[test]
    fn click_maps_through_offset() {
        let mut view = viewport();
        view.ensure_visible(15);
        assert_eq!(press_position(&press(3, 2), &view), Some((8, 3)));
    }

    #[test]
    fn click_on_status_row_ignored() {
        assert_eq!(press_position(&press(3, 10), &viewport()), None);
    }

    #[test]
    fn non_left_press_ignored() {
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 3,
            row: 2,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        assert_eq!(press_position(&event, &viewport()), None);
    }

    #[test]
    fn release_ignored() {
        let event = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 3,
            row: 2,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        assert_eq!(press_position(&event, &viewport()), None);
    }

    #[test]
    fn drag_maps_like_press() {
        assert_eq!(drag_position(&drag(3, 2), &viewport()), Some((2, 3)));
    }

    #[test]
    fn drag_on_status_row_ignored() {
        assert_eq!(drag_position(&drag(3, 10), &viewport()), None);
    }

    #[test]
    fn drag_with_other_button_ignored() {
        let event = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Right),
            column: 3,
            row: 2,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        assert_eq!(drag_position(&event, &viewport()), None);
    }

    #[test]
    fn press_is_not_drag() {
        assert_eq!(drag_position(&press(3, 2), &viewport()), None);
    }

    #[test]
    fn wheel_up_is_negative() {
        let event = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        assert_eq!(wheel_delta(&event), Some(-WHEEL_LINES));
    }

    #[test]
    fn wheel_down_is_positive() {
        let event = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        assert_eq!(wheel_delta(&event), Some(WHEEL_LINES));
    }

    #[test]
    fn press_is_not_wheel() {
        assert_eq!(wheel_delta(&press(3, 2)), None);
    }
}
