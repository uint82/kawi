use std::fs;

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use kawi_buffer::Buffer;
use kawi_tui::frame::{VirtualTextKind, VirtualTextPlacement};

use super::super::MetadataColumns;
use super::{settle, setup};
use crate::app::App;
use crate::keymap::Flavor;
use crate::test_support::temp_dir;

#[test]
fn mouse_maps_metadata_prefix_to_entry_start() {
    let (dir, mut app) = setup("directory_metadata_mouse", &["alpha.txt"], &[]);
    app.sync_active_window();
    let layout = app.window_layout();
    let panes = app.frame_panes(&layout);
    let pane = &panes[0];
    let number = app.editor.buffer_number();
    let width = app
        .directory
        .get(number)
        .unwrap()
        .metadata_width(app.directory_options.metadata);
    assert_eq!(pane.left_width, width);
    assert!(pane.left_width > 0);
    let column = pane.rect.x.saturating_add(app.editor.gutter_width());
    let click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: u16::try_from(column).unwrap(),
        row: u16::try_from(pane.rect.y).unwrap(),
        modifiers: KeyModifiers::NONE,
    };
    app.handle_mouse(click).expect("click filename");
    assert_eq!(app.cursor_position(), (0, 0));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn directory_metadata_loads_with_the_listing() {
    let dir = temp_dir("directory_metadata_load");
    fs::write(dir.join("alpha.txt"), "hello").unwrap();
    let columns = MetadataColumns::parse("permissions,size,modified").unwrap();
    let mut app = App::new(Buffer::from_string(""), Flavor::Vim);
    app.directory_options.metadata = columns;
    assert!(app.open_directory_path(&dir, true));
    settle(&mut app);
    let number = app.editor.buffer_number();
    let state = app.directory.get(number).unwrap();
    let width = state.metadata_width(columns);
    let text = state.metadata_text(0, columns).unwrap();
    assert!(text.starts_with('-'));
    let fields: Vec<&str> = text.split_whitespace().collect();
    assert_eq!(fields.get(1), Some(&"5"));
    let texts = app.directory_metadata_texts(number, &[0]);
    let text = texts[0].as_ref().unwrap();
    assert_eq!(text.kind, VirtualTextKind::Metadata);
    assert_eq!(text.placement, VirtualTextPlacement::BeforeLine);
    let empty = app.directory_metadata_texts(number, &[100]);
    assert_eq!(empty[0].as_ref().unwrap().text, " ".repeat(width));
    let _ = fs::remove_dir_all(dir);
}
