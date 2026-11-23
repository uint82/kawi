use super::*;

use std::collections::HashSet;

#[test]
fn report_covers_the_whole_protocol_universe() {
    let report = api_metadata();
    let method_names: Vec<&str> = report.methods.iter().map(|entry| entry.name).collect();
    let event_names: Vec<&str> = report.events.iter().map(|entry| entry.name).collect();
    assert_eq!(method_names, HOST_METHODS);
    assert_eq!(event_names, EVENT_NAMES);
}

#[test]
fn report_entries_mirror_the_registry() {
    let report = api_metadata();
    for entry in &report.methods {
        let maturity = host_method_maturity(entry.name).unwrap();
        assert_eq!(entry.since, maturity.since, "{}", entry.name);
        assert_eq!(entry.stability, stability_label(maturity), "{}", entry.name);
        assert_eq!(entry.deprecated_since, maturity.deprecated_since);
        assert_eq!(entry.replacement, maturity.replacement);
    }
    for entry in &report.events {
        let maturity = event_maturity(entry.name).unwrap();
        assert_eq!(entry.since, maturity.since, "{}", entry.name);
        assert_eq!(entry.stability, stability_label(maturity), "{}", entry.name);
        assert_eq!(entry.deprecated_since, maturity.deprecated_since);
        assert_eq!(entry.replacement, maturity.replacement);
    }
}

#[test]
fn error_types_are_unique_and_described() {
    let errors = api_metadata().error_types;
    let mut names = HashSet::new();
    let mut codes = HashSet::new();
    for error in &errors {
        assert!(names.insert(error.name), "{}", error.name);
        assert!(codes.insert(error.code), "{}", error.code);
        assert!(!error.description.is_empty(), "{}", error.name);
    }
    assert_eq!(errors.len(), 29);
}

#[test]
fn error_codes_match_the_protocol_constants() {
    let errors = api_metadata().error_types;
    for (name, code) in [
        ("PARSE_ERROR", crate::protocol::PARSE_ERROR),
        ("INVALID_REQUEST", crate::protocol::INVALID_REQUEST),
        ("METHOD_NOT_FOUND", crate::protocol::METHOD_NOT_FOUND),
        ("INVALID_PARAMS", crate::protocol::INVALID_PARAMS),
        ("INTERNAL_ERROR", crate::protocol::INTERNAL_ERROR),
        ("PLUGIN_ERROR", crate::protocol::PLUGIN_ERROR),
        ("STALE_REVISION", crate::protocol::STALE_REVISION),
        ("INVALID_POSITION", crate::protocol::INVALID_POSITION),
        ("CAPABILITY_DENIED", crate::protocol::CAPABILITY_DENIED),
        ("REQUEST_CANCELED", crate::protocol::REQUEST_CANCELED),
        ("PLUGIN_UNAVAILABLE", crate::protocol::PLUGIN_UNAVAILABLE),
        ("UI_BUSY", crate::protocol::UI_BUSY),
        ("JOB_NOT_FOUND", crate::protocol::JOB_NOT_FOUND),
        ("JOB_LIMIT", crate::protocol::JOB_LIMIT),
        ("JOB_CLOSED", crate::protocol::JOB_CLOSED),
        ("JOB_QUEUE_FULL", crate::protocol::JOB_QUEUE_FULL),
        ("PANEL_NOT_FOUND", crate::protocol::PANEL_NOT_FOUND),
        ("OVERLAY_NOT_FOUND", crate::protocol::OVERLAY_NOT_FOUND),
        ("TASK_NOT_FOUND", crate::protocol::TASK_NOT_FOUND),
        ("TASK_LIMIT", crate::protocol::TASK_LIMIT),
        ("TASK_START_FAILED", crate::protocol::TASK_START_FAILED),
        ("BUFFER_NOT_FOUND", crate::protocol::BUFFER_NOT_FOUND),
        ("BUFFER_NOT_ACTIVE", crate::protocol::BUFFER_NOT_ACTIVE),
        ("WINDOW_NOT_FOUND", crate::protocol::WINDOW_NOT_FOUND),
        ("LAST_WINDOW", crate::protocol::LAST_WINDOW),
        ("OPTION_NOT_FOUND", crate::protocol::OPTION_NOT_FOUND),
        ("MODE_NOT_SUPPORTED", crate::protocol::MODE_NOT_SUPPORTED),
        ("BUFFER_MODIFIED", crate::protocol::BUFFER_MODIFIED),
        ("BUFFER_NOT_OWNED", crate::protocol::BUFFER_NOT_OWNED),
    ] {
        let entry = errors
            .iter()
            .find(|error| error.name == name)
            .unwrap_or_else(|| panic!("missing error type: {name}"));
        assert_eq!(entry.code, code, "{name}");
    }
}

#[test]
fn report_version_matches_protocol_constants() {
    let version = api_metadata().version;
    assert_eq!(version.protocol, PROTOCOL_VERSION);
    assert_eq!(version.protocol_minor, PROTOCOL_MINOR);
    assert_eq!(version.process_protocol_minor, PROCESS_PROTOCOL_MINOR);
}

#[test]
fn report_serializes_with_explicit_nulls_for_missing_deprecation_data() {
    let value = serde_json::to_value(api_metadata()).unwrap();
    assert_eq!(value["version"]["protocol"], PROTOCOL_VERSION);
    assert_eq!(
        value["methods"].as_array().unwrap().len(),
        HOST_METHODS.len()
    );
    assert_eq!(value["events"].as_array().unwrap().len(), EVENT_NAMES.len());
    assert_eq!(
        value["error_types"].as_array().unwrap().len(),
        api_metadata().error_types.len()
    );

    let first = &value["methods"][0];
    assert!(first["deprecated_since"].is_null());
    assert!(first["replacement"].is_null());
}

#[test]
fn stability_labels_cover_every_stability() {
    assert_eq!(stability_label(ApiMaturity::stable(1)), "stable");
    assert_eq!(
        stability_label(ApiMaturity::experimental(2)),
        "experimental"
    );
    assert_eq!(
        stability_label(ApiMaturity::deprecated(2, 3, "new.method")),
        "deprecated"
    );
}

#[test]
fn deprecated_entry_keeps_its_version_and_replacement() {
    let maturity = ApiMaturity::deprecated(4, 6, "new.method");
    let entry = entry_metadata("old.method", maturity);
    let value = serde_json::to_value(entry).unwrap();
    assert_eq!(value["name"], "old.method");
    assert_eq!(value["since"], 4);
    assert_eq!(value["stability"], "deprecated");
    assert_eq!(value["deprecated_since"], 6);
    assert_eq!(value["replacement"], "new.method");
}
