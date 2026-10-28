use std::time::{Duration, Instant};

use crossterm::event::KeyCode;
use kawi_editor::Mode;

use super::super::*;
use super::{binding, plain, registry};
use crate::command::CommandRegistry;

#[test]
fn resolver_prefers_complete_matches() {
    let registry = registry();
    let mut resolver = KeymapResolver::new();
    resolver.bind(binding(
        &registry,
        "file.save",
        KeyContext::Normal,
        "gx",
        BindingSource::User,
    ));
    assert_eq!(
        resolver.feed(
            plain(KeyCode::Char('g')),
            KeyContext::Normal,
            Instant::now()
        ),
        ResolveOutcome::Pending
    );
    let outcome = resolver.feed(
        plain(KeyCode::Char('x')),
        KeyContext::Normal,
        Instant::now(),
    );
    match outcome {
        ResolveOutcome::Command(command) => assert_eq!(command.keys.to_string(), "gx"),
        other => panic!("unexpected outcome: {other:?}"),
    }
}

#[test]
fn resolver_replays_mismatched_prefixes() {
    let registry = registry();
    let mut resolver = KeymapResolver::new();
    resolver.bind(binding(
        &registry,
        "file.save",
        KeyContext::Normal,
        "gx",
        BindingSource::User,
    ));
    resolver.feed(
        plain(KeyCode::Char('g')),
        KeyContext::Normal,
        Instant::now(),
    );
    let outcome = resolver.feed(
        plain(KeyCode::Char('z')),
        KeyContext::Normal,
        Instant::now(),
    );
    match outcome {
        ResolveOutcome::Fallback(keys) => {
            assert_eq!(keys.len(), 2);
            assert_eq!(keys[0].code, KeyCodeSpec::Char('g'));
            assert_eq!(keys[1].code, KeyCodeSpec::Char('z'));
        }
        other => panic!("unexpected outcome: {other:?}"),
    }
}

#[test]
fn resolver_passes_through_unbound_keys() {
    let mut resolver = KeymapResolver::new();
    assert_eq!(
        resolver.feed(
            plain(KeyCode::Char('x')),
            KeyContext::Normal,
            Instant::now()
        ),
        ResolveOutcome::Passthrough
    );
}

#[test]
fn resolver_times_out_ambiguous_sequences() {
    let registry = registry();
    let mut resolver = KeymapResolver::new();
    resolver.bind(binding(
        &registry,
        "file.save",
        KeyContext::Normal,
        "g",
        BindingSource::User,
    ));
    resolver.bind(binding(
        &registry,
        "file.quit",
        KeyContext::Normal,
        "gg",
        BindingSource::User,
    ));
    let now = Instant::now();
    assert_eq!(
        resolver.feed(plain(KeyCode::Char('g')), KeyContext::Normal, now),
        ResolveOutcome::Pending
    );
    let outcome = resolver
        .take_timeout(now + Duration::from_secs(2))
        .expect("timeout");
    match outcome {
        ResolveOutcome::Command(command) => assert_eq!(command.keys.to_string(), "g"),
        other => panic!("unexpected outcome: {other:?}"),
    }
}

#[test]
fn resolver_replays_prefix_only_timeouts() {
    let registry = registry();
    let mut resolver = KeymapResolver::new();
    resolver.bind(binding(
        &registry,
        "file.save",
        KeyContext::Normal,
        "gg",
        BindingSource::User,
    ));
    let now = Instant::now();
    resolver.feed(plain(KeyCode::Char('g')), KeyContext::Normal, now);
    let outcome = resolver
        .take_timeout(now + Duration::from_secs(2))
        .expect("timeout");
    match outcome {
        ResolveOutcome::Fallback(keys) => assert_eq!(keys.len(), 1),
        other => panic!("unexpected outcome: {other:?}"),
    }
}

#[test]
fn resolver_respects_source_priority() {
    let registry = registry();
    let mut resolver = KeymapResolver::new();
    resolver.bind(binding(
        &registry,
        "file.save",
        KeyContext::Normal,
        "gx",
        BindingSource::User,
    ));
    resolver.bind(binding(
        &registry,
        "file.quit",
        KeyContext::Normal,
        "gx",
        BindingSource::Plugin,
    ));
    let outcome = resolver.feed(
        plain(KeyCode::Char('g')),
        KeyContext::Normal,
        Instant::now(),
    );
    assert_eq!(outcome, ResolveOutcome::Pending);
    let outcome = resolver.feed(
        plain(KeyCode::Char('x')),
        KeyContext::Normal,
        Instant::now(),
    );
    match outcome {
        ResolveOutcome::Command(command) => {
            assert_eq!(command.command, registry.find("file.save").unwrap())
        }
        other => panic!("unexpected outcome: {other:?}"),
    }
}

#[test]
fn resolver_clears_bindings() {
    let registry = registry();
    let mut resolver = KeymapResolver::new();
    resolver.bind(binding(
        &registry,
        "file.save",
        KeyContext::Normal,
        "gx",
        BindingSource::User,
    ));
    resolver.clear_bindings();
    assert!(resolver.bindings().is_empty());
}

#[test]
fn context_parses_modes() {
    assert_eq!(
        KeyContext::from_mode(Mode::Normal),
        Some(KeyContext::Normal)
    );
    assert_eq!(
        KeyContext::from_mode(Mode::Insert),
        Some(KeyContext::Insert)
    );
    assert_eq!(
        KeyContext::from_mode(Mode::Replace),
        Some(KeyContext::Insert)
    );
    assert_eq!(
        KeyContext::from_mode(Mode::Visual),
        Some(KeyContext::Visual)
    );
    assert_eq!(KeyContext::from_mode(Mode::Command), None);
    assert_eq!(KeyContext::parse("visual"), Some(KeyContext::Visual));
}

#[test]
fn names_resolve_flavors() {
    assert_eq!(super::flavor_from_name("vim"), Some(super::Flavor::Vim));
    assert_eq!(super::flavor_from_name("helix"), Some(super::Flavor::Helix));
}

#[test]
fn unknown_name_resolves_none() {
    assert_eq!(super::flavor_from_name("personal"), None);
    assert_eq!(super::flavor_from_name(""), None);
}

#[test]
fn interrupt_becomes_plain_esc() {
    let interrupt = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(
        super::normalize_interrupt(interrupt),
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
    );
}

#[test]
fn other_keys_pass_through() {
    let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
    assert_eq!(super::normalize_interrupt(key), key);
}
