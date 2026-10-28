use super::*;

fn registry() -> CommandRegistry {
    CommandRegistry::new()
}

#[test]
fn builtin_names_are_unique() {
    let mut names = Vec::new();
    for builtin in builtin::all() {
        names.push(builtin.name);
        names.extend(builtin.aliases.iter().copied());
    }
    names.sort_unstable();
    names.dedup();
    let expected = builtin::all()
        .map(|item| 1 + item.aliases.len())
        .sum::<usize>();
    assert_eq!(names.len(), expected);
}

#[test]
fn finds_names_and_aliases() {
    let registry = registry();
    assert_eq!(registry.find("file.save"), registry.find("save"));
    assert!(registry.find("file.save").is_some());
}

#[test]
fn resolves_special_commands() {
    let registry = registry();
    let none = registry.find("none").expect("none");
    assert_eq!(
        registry.spec(none).expect("none").handler,
        CommandHandler::Noop
    );
    let send = registry.find("keys.send").expect("send");
    assert_eq!(
        registry.spec(send).expect("send").handler,
        CommandHandler::SendKeys
    );
}

#[test]
fn rejects_unknown_commands() {
    assert!(registry().find("plugin.missing").is_none());
}

#[test]
fn action_commands_are_parameterless() {
    let registry = registry();
    let spec = registry
        .spec(registry.find("edit.delete_line").expect("command"))
        .expect("spec");
    assert_eq!(
        spec.handler,
        CommandHandler::Action(kawi_editor::Action::DeleteLine)
    );
}

#[test]
fn names_include_aliases() {
    let registry = registry();
    let names = registry.names();
    assert!(names.contains(&"edit.undo"));
    assert!(names.contains(&"undo"));
    assert!(names.contains(&"quit"));
}
