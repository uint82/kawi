use anyhow::Result;
use kawi_editor::command::{
    classify_parse_failure, suggest_commands, DiagnosticCode as EditorDiagnosticCode, ParseFailure,
    COMMAND_NAMES,
};
use kawi_editor::{parse, Action};

use super::command_text::split_command;
use super::diagnostic::{Diagnostic, DiagnosticSource};
use super::App;
use crate::command::{CommandHandler, CommandId};

impl App {
    pub(super) fn handle_execute(&mut self) -> Result<bool> {
        let text = self.editor.command_text().to_string();
        self.last_command = Some(text.clone());
        self.editor.apply(Action::CancelCommand);
        self.execute_command_text(&text)
    }

    pub(super) fn execute_command_text(&mut self, text: &str) -> Result<bool> {
        let (name, rest) = split_command(text);
        if let Some(id) = self.plugin_command_id(name) {
            return self.execute_plugin_command(id, rest);
        }
        let command = parse(text);
        if command == kawi_editor::Command::Unknown {
            if let Some(failure) = classify_parse_failure(name, rest) {
                self.report_parse_failure(failure);
                return Ok(false);
            }
        }
        self.run_command(command)
    }

    fn report_parse_failure(&mut self, failure: ParseFailure) {
        let diagnostic = self.describe_parse_failure(&failure);
        self.show_diagnostic(diagnostic);
    }

    fn describe_parse_failure(&self, failure: &ParseFailure) -> Diagnostic {
        let code = failure.code();
        match failure {
            ParseFailure::UnknownCommand { name } => {
                let suggestions = self.command_suggestions(name);
                if suggestions.is_empty() {
                    Diagnostic::error(format!("Not an editor command: {name}"))
                        .with_code(code)
                        .with_source(DiagnosticSource::Builtin)
                } else {
                    Diagnostic::error(format!("Not an editor command: {name}."))
                        .with_code(code)
                        .with_source(DiagnosticSource::Builtin)
                        .with_help(format!(
                            "Did you mean {}?",
                            format_suggestions(&suggestions)
                        ))
                }
            }
            ParseFailure::MissingArgument { command } => {
                Diagnostic::error(format!("Argument required: {command}"))
                    .with_code(code)
                    .with_source(DiagnosticSource::Builtin)
            }
            ParseFailure::InvalidArgument { argument } => {
                Diagnostic::error(format!("Invalid argument: {argument}"))
                    .with_code(code)
                    .with_source(DiagnosticSource::Builtin)
            }
        }
    }

    fn command_suggestions(&self, name: &str) -> Vec<String> {
        let mut candidates: Vec<&str> = COMMAND_NAMES.to_vec();
        for spec in self.commands.specs() {
            if matches!(spec.handler, CommandHandler::Plugin(_)) {
                candidates.extend(spec.names());
            }
        }
        suggest_commands(name, &candidates)
    }

    fn plugin_command_id(&self, name: &str) -> Option<CommandId> {
        let id = self.commands.find(name)?;
        matches!(self.commands.spec(id)?.handler, CommandHandler::Plugin(_)).then_some(id)
    }

    fn execute_plugin_command(&mut self, id: CommandId, rest: &str) -> Result<bool> {
        let Some(CommandHandler::Plugin(target)) = self.commands.spec(id).map(|spec| spec.handler)
        else {
            return Ok(false);
        };
        let args = if rest.is_empty() {
            None
        } else if self.plugins.target_accepts_args(target) {
            Some(toml::Value::String(rest.to_string()))
        } else {
            self.show_diagnostic(
                Diagnostic::error(format!("Trailing characters: {rest}"))
                    .with_code(EditorDiagnosticCode::TrailingCharacters)
                    .with_source(DiagnosticSource::Builtin),
            );
            return Ok(false);
        };
        self.dispatch_command_id(id, args)
    }
}

fn format_suggestions(suggestions: &[String]) -> String {
    suggestions
        .iter()
        .map(|suggestion| format!(":{suggestion}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use kawi_buffer::Buffer;

    use super::*;
    use crate::keymap::Flavor;

    fn app() -> App {
        App::new(Buffer::from_string("alpha\nbeta\n"), Flavor::Vim)
    }

    fn execute(app: &mut App, text: &str) {
        app.execute_command_text(text).expect("execute");
    }

    #[test]
    fn reports_unknown_command_with_suggestion() {
        let mut app = app();
        execute(&mut app, "hlep");
        assert_eq!(
            app.last_message(),
            Some("E492: Not an editor command: hlep. Did you mean :help?")
        );
    }

    #[test]
    fn reports_unknown_command_without_suggestion() {
        let mut app = app();
        execute(&mut app, "zzzzzz");
        assert_eq!(
            app.last_message(),
            Some("E492: Not an editor command: zzzzzz")
        );
    }

    #[test]
    fn reports_missing_argument() {
        let mut app = app();
        execute(&mut app, "b");
        assert_eq!(app.last_message(), Some("E471: Argument required: b"));
    }

    #[test]
    fn reports_invalid_argument() {
        let mut app = app();
        execute(&mut app, "messages clear now");
        assert_eq!(
            app.last_message(),
            Some("E474: Invalid argument: clear now")
        );
    }

    #[test]
    fn keeps_silent_for_empty_input() {
        let mut app = app();
        execute(&mut app, "");
        assert_eq!(app.last_message(), None);
    }

    #[test]
    fn does_not_run_a_command_with_trailing_text() {
        let mut app = app();
        execute(&mut app, "q now");
        assert_eq!(app.last_message(), Some("E474: Invalid argument: now"));
    }
}
