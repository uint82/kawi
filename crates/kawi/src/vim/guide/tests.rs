use super::*;
use kawi_editor::Mode;

fn keys(guide: &Guide) -> Vec<char> {
    guide.rows.iter().map(|row| row.key).collect()
}

fn has_key(prefix: &str, mode: Mode, key: char) -> bool {
    guide(prefix, mode).is_some_and(|guide| keys(&guide).contains(&key))
}

fn description(prefix: &str, mode: Mode, key: char) -> &'static str {
    guide(prefix, mode)
        .and_then(|guide| {
            guide
                .rows
                .into_iter()
                .find(|row| row.key == key)
                .map(|row| row.description)
        })
        .expect("guide row")
}

#[test]
fn operator_guide_lists_motions_and_objects() {
    assert!(has_key("d", Mode::Normal, 'w'));
    assert!(has_key("d", Mode::Normal, 'i'));
    assert!(has_key("d", Mode::Normal, 'a'));
    assert!(has_key("di", Mode::Normal, 'w'));
    assert!(has_key("di", Mode::Normal, '('));
    assert!(has_key("dg", Mode::Normal, 'g'));
    assert!(has_key("dg", Mode::Normal, 'E'));
    assert!(!has_key("caw", Mode::Normal, 'w'));
}

#[test]
fn operator_labels_name_the_action() {
    assert_eq!(description("d", Mode::Normal, 'w'), "delete word");
    assert_eq!(description("c", Mode::Normal, 'w'), "change word");
    assert_eq!(description("y", Mode::Normal, 'w'), "yank word");
    assert_eq!(description(">", Mode::Normal, '>'), "indent line");
    assert_eq!(description("<", Mode::Normal, '<'), "unindent line");
    assert_eq!(description("=", Mode::Normal, '='), "reindent line");
}

#[test]
fn object_labels_name_the_scope() {
    assert_eq!(description("di", Mode::Normal, 'w'), "inner word");
    assert_eq!(description("da", Mode::Normal, 'w'), "around word");
    assert_eq!(description("di", Mode::Normal, '('), "inner parentheses");
    assert_eq!(description("da", Mode::Normal, '('), "around parentheses");
    assert_eq!(description("i", Mode::Visual, 'w'), "inner word");
    assert_eq!(description("a", Mode::Visual, 'w'), "around word");
}

#[test]
fn counts_are_ignored_in_prefixes() {
    assert_eq!(guide("2d", Mode::Normal), guide("d", Mode::Normal));
    assert!(guide("2", Mode::Normal).is_none());
    assert!(guide("", Mode::Normal).is_none());
}

#[test]
fn g_guide_lists_commands_and_continuations() {
    for key in ['g', 'd', 'v', 'J', '~', 'u', 'U', 'c'] {
        assert!(has_key("g", Mode::Normal, key), "{key}");
    }
    assert!(has_key("g~", Mode::Normal, 'w'));
    assert!(has_key("g~", Mode::Normal, '~'));
    assert!(has_key("gci", Mode::Normal, '('));
    assert!(has_key("g~g", Mode::Normal, 'g'));
}

#[test]
fn z_and_bracket_guides_list_followups() {
    assert!(has_key("z", Mode::Normal, 'a'));
    assert!(has_key("z", Mode::Normal, 'z'));
    assert!(!has_key("z", Mode::Normal, 'f'));
    assert!(has_key("z", Mode::Visual, 'f'));
    assert!(has_key("[", Mode::Normal, 'c'));
    assert!(has_key("]", Mode::Normal, 'd'));
    assert_eq!(description("z", Mode::Normal, 'a'), "toggle fold");
    assert_eq!(description("z", Mode::Normal, 'o'), "open fold");
    assert_eq!(description("z", Mode::Normal, 'c'), "close fold");
    assert_eq!(description("z", Mode::Normal, 'R'), "open all folds");
    assert_eq!(description("z", Mode::Normal, 'M'), "close all folds");
    assert_eq!(description("[", Mode::Normal, 'c'), "previous hunk");
    assert_eq!(description("]", Mode::Normal, 'c'), "next hunk");
}

#[test]
fn visual_guide_lists_objects_and_g_commands() {
    assert!(has_key("i", Mode::Visual, 'w'));
    assert!(has_key("a", Mode::Visual, '('));
    assert!(has_key("g", Mode::Visual, 'c'));
    assert!(has_key("g", Mode::Visual, '~'));
}

#[test]
fn dynamic_prefixes_show_hints() {
    assert_eq!(
        guide("f", Mode::Normal).unwrap().hint,
        Some("target character")
    );
    assert_eq!(
        guide("df", Mode::Normal).unwrap().hint,
        Some("target character")
    );
    assert_eq!(
        guide("r", Mode::Normal).unwrap().hint,
        Some("replacement character")
    );
    assert_eq!(guide("\"", Mode::Normal).unwrap().hint, Some("register"));
    assert_eq!(guide("m", Mode::Normal).unwrap().hint, Some("mark name"));
    assert_eq!(guide("d'", Mode::Normal).unwrap().hint, Some("mark name"));
    assert_eq!(
        guide("q", Mode::Normal).unwrap().hint,
        Some("macro register")
    );
    assert_eq!(
        guide("@", Mode::Normal).unwrap().hint,
        Some("macro register")
    );
}

#[test]
fn completed_sequences_do_not_show_guides() {
    assert!(guide("diw", Mode::Normal).is_none());
    assert!(guide("dd", Mode::Normal).is_none());
    assert!(guide("g~w", Mode::Normal).is_none());
}

#[test]
fn sequence_tables_match_lookup_functions() {
    for entry in OPERATOR_SEQUENCES {
        assert_eq!(sequence_action(entry.keys), Some(entry.action));
    }
    for entry in OBJECT_TARGETS {
        assert_eq!(object_target(entry.key), Some(entry.object));
        for alias in entry.aliases {
            assert_eq!(object_target(*alias), Some(entry.object));
        }
    }
    for entry in FOLD_COMMANDS {
        assert_eq!(fold_action(entry.key), entry.action);
    }
    for entry in PLACE_COMMANDS {
        assert_eq!(place_action(entry.key), entry.action);
    }
    for entry in G_COMMANDS {
        assert_eq!(g_action(entry.key), entry.action);
    }
    for entry in VISUAL_G_COMMANDS {
        assert_eq!(visual_g_action(entry.key), entry.action);
    }
    for entry in BRACKET_COMMANDS {
        assert_eq!(bracket_action(entry.key, true), Some(entry.forward));
        assert_eq!(bracket_action(entry.key, false), Some(entry.backward));
    }
}

#[test]
fn guide_rows_are_unique() {
    for prefix in [
        "d", "c", "y", "di", "dg", "g", "g~", "gc", "gci", "z", ">", ">g", "[", "]",
    ] {
        let Some(guide) = guide(prefix, Mode::Normal) else {
            continue;
        };
        let mut seen = Vec::new();
        for row in guide.rows {
            assert!(!seen.contains(&row.key), "{prefix}: {}", row.key);
            seen.push(row.key);
        }
    }
}
