use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use kawi_tui::viewport::Viewport;

pub fn press_position(event: &MouseEvent, viewport: &Viewport) -> Option<(usize, usize)> {
    if !is_left_press(event.kind) {
        return None;
    }
    to_buffer_position(event.column, event.row, viewport)
}

fn is_left_press(kind: MouseEventKind) -> bool {
    matches!(kind, MouseEventKind::Down(MouseButton::Left))
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
}
