use super::{languages, servers};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn built_in_language_names_are_unique() {
    let mut names = BTreeSet::new();
    for language in languages() {
        assert!(
            names.insert(language.name.clone()),
            "duplicate language {}",
            language.name
        );
    }
}

#[test]
fn built_in_extensions_are_unique() {
    let mut seen = BTreeMap::new();
    for language in languages() {
        for extension in &language.extensions {
            let key = extension.to_ascii_lowercase();
            if let Some(existing) = seen.insert(key, language.name.clone()) {
                panic!(
                    "extension {extension} is used by {existing} and {}",
                    language.name
                );
            }
        }
    }
}

#[test]
fn built_in_language_servers_are_configured() {
    let servers = servers();
    for language in languages() {
        for server in &language.servers {
            assert!(
                servers.contains_key(server),
                "language {} references unknown server {server}",
                language.name
            );
        }
    }
}

#[test]
fn built_in_servers_are_used() {
    let used: BTreeSet<String> = languages()
        .into_iter()
        .flat_map(|language| language.servers)
        .collect();
    for name in servers().keys() {
        assert!(used.contains(name), "server {name} is not used");
    }
}

#[test]
fn built_in_languages_map_to_standard_servers() {
    let languages = languages();
    let servers_for = |name: &str| {
        languages
            .iter()
            .find(|language| language.name == name)
            .expect("language")
            .servers
            .clone()
    };
    assert_eq!(servers_for("rust"), vec!["rust-analyzer"]);
    assert_eq!(servers_for("python"), vec!["pyright"]);
}
