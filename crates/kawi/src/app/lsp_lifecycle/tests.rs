use crate::keymap::Flavor;
use crate::App;
use kawi_buffer::Buffer;

fn app() -> App {
    App::new(Buffer::from_string(""), Flavor::Vim)
}

#[test]
fn lifecycle_requires_a_file_or_server() {
    let mut app = app();
    app.execute_command_text("lsp stop").unwrap();
    assert!(app
        .last_message()
        .is_some_and(|message| message.contains("requires a file or server")));
}

#[test]
fn lifecycle_reports_a_missing_service() {
    let mut app = app();
    app.execute_command_text("lsp start fake").unwrap();
    assert!(app
        .last_message()
        .is_some_and(|message| message.contains("service is not running")));
}

#[test]
fn builtin_lifecycle_commands_require_a_file() {
    let mut app = app();
    for name in ["lsp.start", "lsp.stop", "lsp.restart"] {
        let id = app.commands.find(name).expect("command");
        app.dispatch_command_id(id, None).unwrap();
        assert!(app
            .last_message()
            .is_some_and(|message| message.contains("requires a file or server")));
    }
}
