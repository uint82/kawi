use kawi_editor::command::CommandCompletion;
use kawi_help::HelpIndex;
use std::collections::BTreeMap;

use super::help::{COMMANDS_TOPIC, OPTIONS_TOPIC, OPTION_TOPIC};
use super::App;
use crate::options::{self, OptionContext, OptionKind, OptionSpec};

impl App {
    pub(super) fn refresh_command_completion(&mut self) {
        let (option_values, boolean_options) = self.option_completions();
        let completion = CommandCompletion::new(option_values, boolean_options, help_topics());
        self.editor.set_command_completion(completion);
    }

    fn option_completions(&self) -> (BTreeMap<String, Vec<String>>, Vec<String>) {
        let context = self.option_context();
        let mut option_values = BTreeMap::new();
        let mut boolean_options = Vec::new();
        for spec in options::OPTIONS {
            add_option_completion(&context, spec, &mut option_values, &mut boolean_options);
        }
        (option_values, boolean_options)
    }
}

fn add_option_completion(
    context: &OptionContext<'_>,
    spec: OptionSpec,
    option_values: &mut BTreeMap<String, Vec<String>>,
    boolean_options: &mut Vec<String>,
) {
    if matches!(spec.kind, OptionKind::Bool) {
        boolean_options.push(spec.name.to_string());
    }
    let values = spec.values(context);
    if values.is_empty() {
        return;
    }
    for name in std::iter::once(spec.name).chain(spec.aliases.iter().copied()) {
        option_values.insert(name.to_string(), values.clone());
    }
}

fn help_topics() -> Vec<String> {
    let mut topics: Vec<String> = HelpIndex
        .pages()
        .flat_map(|page| std::iter::once(page.slug).chain(page.aliases.iter().copied()))
        .map(str::to_string)
        .collect();
    topics.extend(
        [COMMANDS_TOPIC, OPTION_TOPIC, OPTIONS_TOPIC]
            .into_iter()
            .map(str::to_string),
    );
    topics.sort();
    topics.dedup();
    topics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Flavor;
    use kawi_buffer::Buffer;
    use kawi_editor::Action;
    use std::fs;
    use std::path::PathBuf;

    fn app() -> App {
        App::new(Buffer::from_string("alpha\n"), Flavor::Vim)
    }

    fn type_command(app: &mut App, text: &str) {
        app.editor.apply(Action::EnterCommand);
        for character in text.chars() {
            app.editor.apply(Action::PushCommandChar(character));
        }
    }

    fn complete(app: &mut App) {
        app.editor.apply(Action::CompleteCommand);
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kawi_command_completion_{}_{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn completes_builtin_theme_values() {
        let mut app = app();
        type_command(&mut app, "set theme ");
        complete(&mut app);
        assert_eq!(app.editor.command_text(), "set theme classic");
    }

    #[test]
    fn completes_custom_theme_values() {
        let dir = temp_dir("custom_theme");
        fs::write(
            dir.join("personal.toml"),
            crate::test_support::custom_theme(),
        )
        .unwrap();
        let mut app = app();
        app.theme_dir = Some(dir.clone());
        app.refresh_command_completion();
        type_command(&mut app, "set theme per");
        complete(&mut app);
        assert_eq!(app.editor.command_text(), "set theme personal");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn completes_values_for_option_aliases() {
        let mut app = app();
        type_command(&mut app, "set scl ");
        complete(&mut app);
        assert_eq!(app.editor.command_text(), "set scl always");
    }

    #[test]
    fn completes_boolean_negations_from_metadata() {
        let mut app = app();
        type_command(&mut app, "set no");
        complete(&mut app);
        assert_eq!(app.editor.command_text(), "set noauto_completion");
    }

    #[test]
    fn completes_help_topics_from_help_index() {
        let mut app = app();
        type_command(&mut app, "help configur");
        complete(&mut app);
        assert_eq!(app.editor.command_text(), "help configuration");
    }

    #[test]
    fn completes_help_option_names() {
        let mut app = app();
        type_command(&mut app, "help option signcol");
        complete(&mut app);
        assert_eq!(app.editor.command_text(), "help option signcolumn");
    }
}
