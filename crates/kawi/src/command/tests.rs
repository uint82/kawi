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
fn removed_plugin_commands_are_hidden_and_ids_are_reused() {
    let mut registry = registry();
    let id = registry
        .register_plugin(
            "demo.ping".to_string(),
            "Ping".to_string(),
            "demo".to_string(),
            2,
        )
        .expect("register");
    registry.remove_plugin("demo");
    assert!(registry.find("demo.ping").is_none());
    assert!(registry.spec(id).is_none());
    assert!(!registry.specs().any(|spec| spec.name == "demo.ping"));
    let reused = registry
        .register_plugin(
            "demo.ping".to_string(),
            "Ping again".to_string(),
            "demo".to_string(),
            3,
        )
        .expect("re-register");
    assert_eq!(id, reused);
    assert_eq!(
        registry.spec(reused).expect("spec").handler,
        CommandHandler::Plugin(3)
    );
}

#[test]
fn removing_a_plugin_keeps_builtins_and_other_plugins() {
    let mut registry = registry();
    registry
        .register_plugin(
            "demo.ping".to_string(),
            "Ping".to_string(),
            "demo".to_string(),
            0,
        )
        .expect("demo");
    registry
        .register_plugin(
            "other.ping".to_string(),
            "Ping".to_string(),
            "other".to_string(),
            1,
        )
        .expect("other");
    registry.remove_plugin("demo");
    assert!(registry.find("demo.ping").is_none());
    assert!(registry.find("other.ping").is_some());
    assert!(registry.find("file.save").is_some());
}
