use super::*;
use crate::actions::Action;
use crate::mode::Mode;
use kawi_buffer::Buffer;

fn editor_with(text: &str) -> Editor {
    Editor::new(Buffer::from_string(text))
}

fn tall_editor() -> Editor {
    let text = (1..=100)
        .map(|i| format!("    {i} abcdef"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut e = editor_with(&text);
    e.set_view_size(80, 20);
    e
}

fn place_top(e: &mut Editor, top: usize) {
    e.viewport.place_top(top, e.buffer.line_count());
}

fn push_digits(e: &mut Editor, digits: &[u8]) {
    for digit in digits {
        e.apply(Action::PushCount(*digit));
    }
}

fn wide_lines() -> Editor {
    let text = (0..40)
        .map(|i| format!("{i:02} abcdefghijklmnopqrstuvwxyz"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut e = editor_with(&text);
    e.set_view_size(80, 20);
    e
}

#[test]
fn line_down_moves_top_and_cursor_at_top() {
    let mut e = wide_lines();
    e.apply(Action::ScrollLineDown);
    assert_eq!(e.view_top(), 1);
    assert_eq!(e.cursor.row(), 1);
}

#[test]
fn line_down_keeps_visible_cursor() {
    let mut e = wide_lines();
    for _ in 0..10 {
        e.apply(Action::MoveDown);
    }
    e.apply(Action::ScrollLineDown);
    assert_eq!(e.view_top(), 1);
    assert_eq!(e.cursor.row(), 10);
}

#[test]
fn line_down_honors_count() {
    let mut e = wide_lines();
    push_digits(&mut e, &[3]);
    e.apply(Action::ScrollLineDown);
    assert_eq!(e.view_top(), 3);
    assert_eq!(e.cursor.row(), 3);
}

#[test]
fn line_down_over_scrolls_to_last_line() {
    let mut e = wide_lines();
    push_digits(&mut e, &[9, 9]);
    e.apply(Action::ScrollLineDown);
    assert_eq!(e.view_top(), 39);
    assert_eq!(e.cursor.row(), 39);
}

#[test]
fn line_down_stops_at_last_line() {
    let mut e = wide_lines();
    e.apply(Action::GotoLastLine);
    let top = e.view_top();
    e.apply(Action::ScrollLineDown);
    assert_eq!(e.view_top(), top.saturating_add(1));
    assert_eq!(e.cursor.row(), 39);
}

#[test]
fn line_up_moves_top_and_keeps_visible_cursor() {
    let mut e = wide_lines();
    place_top(&mut e, 10);
    e.apply(Action::JumpTo(15, 0));
    place_top(&mut e, 10);
    e.apply(Action::ScrollLineUp);
    assert_eq!(e.view_top(), 9);
    assert_eq!(e.cursor.row(), 15);
}

#[test]
fn line_up_pulls_cursor_into_view() {
    let mut e = wide_lines();
    place_top(&mut e, 5);
    e.apply(Action::JumpTo(24, 0));
    let top = e.view_top();
    e.apply(Action::ScrollLineUp);
    assert_eq!(e.view_top(), top.saturating_sub(1));
    assert_eq!(e.cursor.row(), top.saturating_add(18));
}

#[test]
fn line_up_restores_desired_column() {
    let text = (0..30)
        .map(|i| {
            if i == 23 {
                "ab".to_string()
            } else {
                format!("{i:02} abcdefghijklmnopqrstuvwxyz")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut e = editor_with(&text);
    e.set_view_size(80, 20);
    e.apply(Action::JumpTo(25, 5));
    place_top(&mut e, 5);
    e.apply(Action::ScrollLineUp);
    assert_eq!(e.cursor.row(), 23);
    assert_eq!(e.cursor.col(), 1);
}

#[test]
fn line_up_stops_at_first_line() {
    let mut e = wide_lines();
    e.apply(Action::ScrollLineUp);
    assert_eq!(e.view_top(), 0);
    assert_eq!(e.cursor.row(), 0);
}

#[test]
fn half_down_moves_view_and_cursor() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 7));
    e.apply(Action::ScrollHalfDown);
    assert_eq!((e.row(), e.col()), (59, 7));
    assert_eq!(e.view_top(), 50);
}

#[test]
fn half_up_moves_view_and_cursor() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 7));
    e.apply(Action::ScrollHalfUp);
    assert_eq!((e.row(), e.col()), (39, 7));
    assert_eq!(e.view_top(), 30);
}

#[test]
fn half_down_pins_cursor_to_bottom() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(59, 0));
    e.apply(Action::ScrollHalfDown);
    assert_eq!((e.row(), e.col()), (69, 0));
    assert_eq!(e.view_top(), 50);
}

#[test]
fn half_up_pins_cursor_to_top() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(40, 0));
    e.apply(Action::ScrollHalfUp);
    assert_eq!((e.row(), e.col()), (30, 0));
    assert_eq!(e.view_top(), 30);
}

#[test]
fn half_up_at_top_clamps_cursor() {
    let mut e = tall_editor();
    e.apply(Action::JumpTo(5, 6));
    e.apply(Action::ScrollHalfUp);
    assert_eq!((e.row(), e.col()), (0, 6));
    assert_eq!(e.view_top(), 0);
}

#[test]
fn half_down_at_bottom_clamps_cursor() {
    let mut e = tall_editor();
    place_top(&mut e, 80);
    e.apply(Action::JumpTo(94, 6));
    e.apply(Action::ScrollHalfDown);
    assert_eq!((e.row(), e.col()), (99, 6));
    assert_eq!(e.view_top(), 80);
}

#[test]
fn half_down_uses_count() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    push_digits(&mut e, &[5]);
    e.apply(Action::ScrollHalfDown);
    assert_eq!((e.row(), e.col()), (54, 0));
    assert_eq!(e.view_top(), 45);
}

#[test]
fn half_down_reuses_counted_amount() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    push_digits(&mut e, &[5]);
    e.apply(Action::ScrollHalfDown);
    e.apply(Action::ScrollHalfDown);
    assert_eq!((e.row(), e.col()), (59, 0));
    assert_eq!(e.view_top(), 50);
}

#[test]
fn half_up_reuses_counted_amount() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    push_digits(&mut e, &[7]);
    e.apply(Action::ScrollHalfUp);
    e.apply(Action::ScrollHalfUp);
    assert_eq!((e.row(), e.col()), (35, 0));
    assert_eq!(e.view_top(), 26);
}

#[test]
fn half_scroll_amount_shared_between_directions() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    push_digits(&mut e, &[5]);
    e.apply(Action::ScrollHalfDown);
    e.apply(Action::ScrollHalfUp);
    assert_eq!((e.row(), e.col()), (49, 0));
    assert_eq!(e.view_top(), 40);
}

#[test]
fn half_scroll_count_clamps_to_view_height() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    push_digits(&mut e, &[9, 9]);
    e.apply(Action::ScrollHalfDown);
    e.apply(Action::ScrollHalfDown);
    assert_eq!((e.row(), e.col()), (89, 0));
    assert_eq!(e.view_top(), 80);
}

#[test]
fn resize_resets_scroll_amount() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    push_digits(&mut e, &[5]);
    e.apply(Action::ScrollHalfDown);
    e.set_view_size(80, 40);
    e.apply(Action::ScrollHalfDown);
    assert_eq!((e.row(), e.col()), (74, 0));
    assert_eq!(e.view_top(), 60);
}

#[test]
fn width_change_keeps_scroll_amount() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    push_digits(&mut e, &[5]);
    e.apply(Action::ScrollHalfDown);
    e.set_view_size(60, 20);
    e.apply(Action::ScrollHalfDown);
    assert_eq!((e.row(), e.col()), (59, 0));
    assert_eq!(e.view_top(), 50);
}

#[test]
fn page_scroll_keeps_scroll_amount() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    push_digits(&mut e, &[5]);
    e.apply(Action::ScrollHalfDown);
    e.apply(Action::ScrollPageDown);
    e.apply(Action::ScrollHalfDown);
    assert_eq!((e.row(), e.col()), (68, 0));
    assert_eq!(e.view_top(), 68);
}

#[test]
fn page_down_moves_cursor_to_first_line() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    e.apply(Action::ScrollPageDown);
    assert_eq!((e.row(), e.col()), (58, 0));
    assert_eq!(e.view_top(), 58);
}

#[test]
fn page_up_moves_cursor_to_last_line() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    e.apply(Action::ScrollPageUp);
    assert_eq!((e.row(), e.col()), (41, 0));
    assert_eq!(e.view_top(), 22);
}

#[test]
fn page_down_uses_count() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    push_digits(&mut e, &[2]);
    e.apply(Action::ScrollPageDown);
    assert_eq!((e.row(), e.col()), (76, 0));
    assert_eq!(e.view_top(), 76);
}

#[test]
fn page_down_near_end_snaps_to_last_line() {
    let mut e = tall_editor();
    place_top(&mut e, 80);
    e.apply(Action::JumpTo(80, 0));
    e.apply(Action::ScrollPageDown);
    assert_eq!((e.row(), e.col()), (99, 0));
    assert_eq!(e.view_top(), 99);
}

#[test]
fn page_up_near_start_lands_on_first_line() {
    let mut e = tall_editor();
    place_top(&mut e, 5);
    e.apply(Action::JumpTo(14, 0));
    e.apply(Action::ScrollPageUp);
    assert_eq!((e.row(), e.col()), (19, 0));
    assert_eq!(e.view_top(), 0);
}

#[test]
fn page_scroll_without_room_keeps_cursor() {
    let mut e = tall_editor();
    e.apply(Action::JumpTo(4, 0));
    e.apply(Action::ScrollPageUp);
    assert_eq!((e.row(), e.col()), (4, 0));
    assert_eq!(e.view_top(), 0);
}

#[test]
fn viewport_top_middle_bottom_keep_column() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 7));
    e.apply(Action::MoveToViewportTop);
    assert_eq!((e.row(), e.col()), (40, 7));
    e.apply(Action::JumpTo(49, 7));
    e.apply(Action::MoveToViewportMiddle);
    assert_eq!((e.row(), e.col()), (49, 7));
    e.apply(Action::JumpTo(49, 7));
    e.apply(Action::MoveToViewportBottom);
    assert_eq!((e.row(), e.col()), (59, 7));
    assert_eq!(e.view_top(), 40);
}

#[test]
fn viewport_top_uses_count() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    push_digits(&mut e, &[2]);
    e.apply(Action::MoveToViewportTop);
    assert_eq!((e.row(), e.col()), (41, 0));
}

#[test]
fn viewport_bottom_clamps_to_top() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    push_digits(&mut e, &[2, 1]);
    e.apply(Action::MoveToViewportBottom);
    assert_eq!((e.row(), e.col()), (40, 0));
}

#[test]
fn viewport_middle_ignores_count() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    push_digits(&mut e, &[3]);
    e.apply(Action::MoveToViewportMiddle);
    assert_eq!((e.row(), e.col()), (49, 0));
}

#[test]
fn viewport_middle_of_short_file() {
    let mut e = editor_with("    1\n    2\n    3");
    e.set_view_size(80, 20);
    e.apply(Action::JumpTo(0, 0));
    e.apply(Action::MoveToViewportMiddle);
    assert_eq!((e.row(), e.col()), (1, 0));
}

#[test]
fn place_top_bottom_center() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    e.apply(Action::PlaceTop);
    assert_eq!(e.view_top(), 49);
    assert_eq!((e.row(), e.col()), (49, 0));
    e.apply(Action::PlaceBottom);
    assert_eq!(e.view_top(), 30);
    assert_eq!((e.row(), e.col()), (49, 0));
    e.apply(Action::PlaceCenter);
    assert_eq!(e.view_top(), 40);
    assert_eq!((e.row(), e.col()), (49, 0));
}

#[test]
fn place_center_near_end() {
    let mut e = tall_editor();
    e.apply(Action::JumpTo(94, 6));
    e.apply(Action::PlaceCenter);
    assert_eq!(e.view_top(), 85);
    assert_eq!((e.row(), e.col()), (94, 6));
}

#[test]
fn place_top_near_end_sticks() {
    let mut e = tall_editor();
    e.apply(Action::JumpTo(94, 6));
    e.apply(Action::PlaceTop);
    assert_eq!(e.view_top(), 94);
    assert_eq!((e.row(), e.col()), (94, 6));
}

#[test]
fn place_bottom_near_end() {
    let mut e = tall_editor();
    e.apply(Action::JumpTo(94, 6));
    e.apply(Action::PlaceBottom);
    assert_eq!(e.view_top(), 75);
    assert_eq!((e.row(), e.col()), (94, 6));
}

#[test]
fn place_center_uses_count() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    push_digits(&mut e, &[3, 0]);
    e.apply(Action::PlaceCenter);
    assert_eq!((e.row(), e.col()), (29, 0));
    assert_eq!(e.view_top(), 20);
}

#[test]
fn visual_scroll_extends_head() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    e.apply(Action::EnterVisual);
    e.apply(Action::MoveToViewportTop);
    assert_eq!(e.selection_range(), Some(((40, 0), (49, 0))));
    assert_eq!((e.row(), e.col()), (40, 0));
    assert_eq!(e.mode(), Mode::Visual);
}

#[test]
fn visual_half_scroll_extends_head() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(49, 0));
    e.apply(Action::EnterVisual);
    e.apply(Action::ScrollHalfDown);
    assert_eq!(e.selection_range(), Some(((49, 0), (59, 0))));
    assert_eq!((e.row(), e.col()), (59, 0));
    assert_eq!(e.view_top(), 50);
}

#[test]
fn wheel_scroll_pins_cursor() {
    let mut e = tall_editor();
    place_top(&mut e, 40);
    e.apply(Action::JumpTo(59, 3));
    e.scroll_wheel(3);
    assert_eq!(e.view_top(), 43);
    assert_eq!((e.row(), e.col()), (59, 3));
    e.scroll_wheel(30);
    assert_eq!(e.view_top(), 73);
    assert_eq!((e.row(), e.col()), (73, 3));
}

#[test]
fn reveal_cursor_centers_far_jumps() {
    let mut e = tall_editor();
    e.cursor.set_position(50, 0);
    e.reveal_cursor();
    assert_eq!(e.view_top(), 41);
}

fn forty_line_editor() -> Editor {
    let text = (0..40)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut e = editor_with(&text);
    e.set_view_size(80, 8);
    e
}

#[test]
fn linewise_delete_centers_view_when_cursor_lands_far_above_top() {
    let mut e = forty_line_editor();
    e.apply(Action::JumpTo(8, 0));
    e.apply(Action::EnterVisualLine);
    e.apply(Action::JumpTo(12, 0));
    place_top(&mut e, 12);
    e.apply(Action::DeleteSelection);
    assert_eq!((e.view_top(), e.row()), (5, 8));
}

#[test]
fn linewise_delete_places_view_top_when_cursor_lands_near_above_top() {
    let mut e = forty_line_editor();
    e.apply(Action::JumpTo(8, 0));
    e.apply(Action::EnterVisualLine);
    e.apply(Action::JumpTo(10, 0));
    place_top(&mut e, 10);
    e.apply(Action::DeleteSelection);
    assert_eq!((e.view_top(), e.row()), (8, 8));
}
