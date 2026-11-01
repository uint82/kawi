use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use serde_json::json;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn input_params() -> UiInputParams {
    UiInputParams {
        title: "Name".to_string(),
        placeholder: None,
        value: None,
        max_length: None,
    }
}

fn select_params() -> UiSelectParams {
    UiSelectParams {
        title: "Pick".to_string(),
        options: vec!["alpha".to_string(), "beta".to_string()],
        selected: None,
    }
}

fn progress_params() -> UiProgressStartParams {
    UiProgressStartParams {
        title: "Index".to_string(),
        message: Some("working".to_string()),
        fraction: Some(0.5),
    }
}

#[test]
fn input_prompt_returns_typed_text() {
    let mut ui = PluginUi::default();
    ui.open_input("demo", json!(7), input_params()).unwrap();
    for character in "abc".chars() {
        assert!(ui.handle_key(key(KeyCode::Char(character))).is_none());
    }
    let completion = ui.handle_key(key(KeyCode::Enter)).unwrap();
    assert_eq!(completion.plugin, "demo");
    assert_eq!(completion.request, json!(7));
    assert_eq!(completion.outcome, UiPromptOutcome::Text("abc".to_string()));
    assert!(!ui.has_prompt());
}

#[test]
fn input_prompt_respects_max_length() {
    let mut ui = PluginUi::default();
    ui.open_input(
        "demo",
        json!(1),
        UiInputParams {
            max_length: Some(2),
            value: Some("ab".to_string()),
            ..input_params()
        },
    )
    .unwrap();
    ui.handle_key(key(KeyCode::Char('c')));
    let completion = ui.handle_key(key(KeyCode::Enter)).unwrap();
    assert_eq!(completion.outcome, UiPromptOutcome::Text("ab".to_string()));
}

#[test]
fn input_prompt_cancels() {
    let mut ui = PluginUi::default();
    ui.open_input("demo", json!(1), input_params()).unwrap();
    let completion = ui.handle_key(key(KeyCode::Esc)).unwrap();
    assert_eq!(completion.outcome, UiPromptOutcome::Canceled);
}

#[test]
fn select_prompt_returns_the_chosen_option() {
    let mut ui = PluginUi::default();
    ui.open_select(
        "demo",
        json!(1),
        UiSelectParams {
            selected: Some(1),
            ..select_params()
        },
    )
    .unwrap();
    let completion = ui.handle_key(key(KeyCode::Enter)).unwrap();
    assert_eq!(
        completion.outcome,
        UiPromptOutcome::Selected("beta".to_string())
    );
}

#[test]
fn select_prompt_cancels() {
    let mut ui = PluginUi::default();
    ui.open_select("demo", json!(1), select_params()).unwrap();
    let completion = ui.handle_key(key(KeyCode::Esc)).unwrap();
    assert_eq!(completion.outcome, UiPromptOutcome::Canceled);
}

#[test]
fn confirm_prompt_returns_a_boolean() {
    let mut ui = PluginUi::default();
    ui.open_confirm(
        "demo",
        json!(1),
        UiConfirmParams {
            title: "Continue".to_string(),
            message: None,
        },
    )
    .unwrap();
    let completion = ui.handle_key(key(KeyCode::Char('y'))).unwrap();
    assert_eq!(completion.outcome, UiPromptOutcome::Confirmed(true));

    ui.open_confirm(
        "demo",
        json!(2),
        UiConfirmParams {
            title: "Continue".to_string(),
            message: None,
        },
    )
    .unwrap();
    let completion = ui.handle_key(key(KeyCode::Char('n'))).unwrap();
    assert_eq!(completion.outcome, UiPromptOutcome::Confirmed(false));
}

#[test]
fn progress_updates_and_finishes() {
    let mut ui = PluginUi::default();
    let id = ui.start_progress("demo", progress_params()).unwrap();
    assert_eq!(
        ui.progress_text().as_deref(),
        Some("Index: working 50% [plugin.cancel]")
    );
    assert!(!ui
        .update_progress(UiProgressUpdateParams {
            id,
            message: Some("half".to_string()),
            fraction: Some(0.25),
        })
        .unwrap());
    assert_eq!(
        ui.progress_text().as_deref(),
        Some("Index: half 25% [plugin.cancel]")
    );
    ui.finish_progress(UiProgressFinishParams { id }).unwrap();
    assert!(ui.progress_text().is_none());
}

#[test]
fn progress_can_be_canceled() {
    let mut ui = PluginUi::default();
    let id = ui.start_progress("demo", progress_params()).unwrap();
    assert_eq!(ui.cancel_progress(), Some(("demo".to_string(), id)));
    assert!(ui.progress_text().is_none());
    assert!(ui
        .update_progress(UiProgressUpdateParams {
            id,
            message: None,
            fraction: None,
        })
        .unwrap());
    assert!(ui.finish_progress(UiProgressFinishParams { id }).is_ok());
}

#[test]
fn rejects_an_active_prompt_and_progress() {
    let mut ui = PluginUi::default();
    ui.open_input("demo", json!(1), input_params()).unwrap();
    assert_eq!(
        ui.open_confirm(
            "demo",
            json!(2),
            UiConfirmParams {
                title: "Continue".to_string(),
                message: None,
            },
        )
        .unwrap_err()
        .code,
        kawi_plugin::UI_BUSY
    );
    ui.handle_key(key(KeyCode::Esc));
    ui.start_progress("demo", progress_params()).unwrap();
    assert_eq!(
        ui.start_progress("demo", progress_params())
            .unwrap_err()
            .code,
        kawi_plugin::UI_BUSY
    );
}

#[test]
fn rejects_invalid_progress_fractions() {
    let mut ui = PluginUi::default();
    for fraction in [-0.1, 1.1, f32::NAN] {
        assert_eq!(
            ui.start_progress(
                "demo",
                UiProgressStartParams {
                    fraction: Some(fraction),
                    ..progress_params()
                },
            )
            .unwrap_err()
            .code,
            kawi_plugin::INVALID_PARAMS
        );
    }
}

#[test]
fn rejects_invalid_input_limits() {
    for max_length in [0, 4097] {
        let mut ui = PluginUi::default();
        assert_eq!(
            ui.open_input(
                "demo",
                json!(1),
                UiInputParams {
                    max_length: Some(max_length),
                    ..input_params()
                },
            )
            .unwrap_err()
            .code,
            kawi_plugin::INVALID_PARAMS
        );
    }
}

#[test]
fn rejects_duplicate_select_options() {
    let mut ui = PluginUi::default();
    assert_eq!(
        ui.open_select(
            "demo",
            json!(1),
            UiSelectParams {
                options: vec!["alpha".to_string(), "alpha".to_string()],
                ..select_params()
            },
        )
        .unwrap_err()
        .code,
        kawi_plugin::INVALID_PARAMS
    );
}

#[test]
fn rejects_oversized_select_options() {
    let mut ui = PluginUi::default();
    assert_eq!(
        ui.open_select(
            "demo",
            json!(1),
            UiSelectParams {
                options: vec!["x".repeat(513)],
                ..select_params()
            },
        )
        .unwrap_err()
        .code,
        kawi_plugin::INVALID_PARAMS
    );
}

#[test]
fn removing_a_plugin_clears_its_ui() {
    let mut ui = PluginUi::default();
    ui.open_input("demo", json!(4), input_params()).unwrap();
    ui.start_progress("demo", progress_params()).unwrap();
    assert_eq!(
        ui.remove_plugin("demo"),
        Some(("demo".to_string(), json!(4)))
    );
    assert!(!ui.has_prompt());
    assert!(ui.progress_text().is_none());
}
