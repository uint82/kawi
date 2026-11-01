use std::collections::BTreeSet;

use super::expectations::{
    parse, validate, Expectation, ExpectationFile, PROCESS_EXPECTATIONS, TEST_RESOLVER_ROWS,
    WASM_EXPECTATIONS,
};
use super::harness::INVOCATION_TIMEOUT;
use super::report::Report;
use super::scenario::{
    all_events, all_methods, covered_events, covered_methods, scenarios, Uncovered,
    UNCOVERED_EVENTS, UNCOVERED_METHODS,
};

const LANDED_FEATURES: &[&str] = &[];

const SHORT_TIMEOUT_MS: u64 = 2_000;
const DEFAULT_TIMEOUT_FLOOR_MS: u64 = 5_000;

#[test]
fn scenario_ids_are_unique_and_not_empty() {
    let mut ids = BTreeSet::new();
    for scenario in scenarios() {
        assert!(!scenario.id.trim().is_empty());
        assert!(
            ids.insert(scenario.id),
            "duplicate scenario: {}",
            scenario.id
        );
    }
}

#[test]
fn timeout_rows_keep_a_short_budget_and_normal_rows_have_headroom() {
    let timeout = scenarios()
        .iter()
        .find(|scenario| scenario.id == "trap.timeout")
        .expect("timeout scenario");
    assert_eq!(timeout.timeout_ms(), SHORT_TIMEOUT_MS);
    let normal = scenarios()
        .iter()
        .find(|scenario| scenario.id == "core.echo")
        .expect("normal scenario");
    assert!(normal.timeout_ms() >= DEFAULT_TIMEOUT_FLOOR_MS);
}

#[test]
fn invocation_timeout_exceeds_every_scenario_timeout() {
    let max_timeout = scenarios()
        .iter()
        .map(|scenario| scenario.timeout_ms())
        .max()
        .expect("scenarios");
    assert!(INVOCATION_TIMEOUT.as_millis() > u128::from(max_timeout));
}

#[test]
fn both_runtimes_have_exactly_one_status_for_every_scenario() {
    for (name, text) in [
        ("process", PROCESS_EXPECTATIONS),
        ("wasm", WASM_EXPECTATIONS),
    ] {
        let file = parse(name, text);
        let problems = validate(scenarios(), &file);
        assert!(problems.is_empty(), "{name}: {problems:#?}");
    }
}

#[test]
fn not_supported_entries_have_a_kind_and_reason() {
    for (name, text) in [
        ("process", PROCESS_EXPECTATIONS),
        ("wasm", WASM_EXPECTATIONS),
    ] {
        let file = parse(name, text);
        for (id, expectation) in &file.rows {
            if let Expectation::NotSupported {
                kind,
                reason,
                issue,
                ..
            } = expectation
            {
                assert!(!kind.trim().is_empty(), "{name} {id}: empty kind");
                assert!(!reason.trim().is_empty(), "{name} {id}: empty reason");
                assert!(
                    issue
                        .as_deref()
                        .is_none_or(|issue| !issue.trim().is_empty()),
                    "{name} {id}: empty issue"
                );
            }
        }
    }
}

#[cfg(not(all(feature = "wasm-test-network", debug_assertions)))]
#[test]
fn scripted_network_rows_require_the_test_resolver() {
    let file = parse("wasm", WASM_EXPECTATIONS);
    for id in TEST_RESOLVER_ROWS {
        let expectation = file.rows.get(*id).expect("scripted network row");
        match expectation {
            Expectation::NotSupported { kind, .. } => assert_eq!(kind, "TestResolverRequired"),
            Expectation::Pass => panic!("{id} passes without the test resolver"),
        }
    }
}

#[cfg(all(feature = "wasm-test-network", debug_assertions))]
#[test]
fn scripted_network_rows_pass_with_the_test_resolver() {
    let file = parse("wasm", WASM_EXPECTATIONS);
    for id in TEST_RESOLVER_ROWS {
        let expectation = file.rows.get(*id).expect("scripted network row");
        assert!(
            expectation.is_pass(),
            "{id} is not run with the test resolver"
        );
    }
}

#[test]
fn temporary_not_supported_fails_once_its_feature_lands() {
    for (name, text) in [
        ("process", PROCESS_EXPECTATIONS),
        ("wasm", WASM_EXPECTATIONS),
    ] {
        let file = parse(name, text);
        for (id, expectation) in &file.rows {
            let Expectation::NotSupported {
                temporary, feature, ..
            } = expectation
            else {
                continue;
            };
            assert!(
                !temporary
                    || feature
                        .as_deref()
                        .is_none_or(|feature| !LANDED_FEATURES.contains(&feature)),
                "{name} {id}: temporary not-supported outlived its feature"
            );
        }
    }
}

#[test]
fn enforcement_table_is_covered_or_explicitly_uncovered() {
    assert_coverage(
        "method",
        all_methods(),
        covered_methods(),
        UNCOVERED_METHODS,
    );
    assert_coverage("event", all_events(), covered_events(), UNCOVERED_EVENTS);
}

#[test]
fn report_reports_missing_wasm_as_not_built() {
    let mut report = Report::default();
    report.record_not_built(super::harness::Runtime::Wasm, "wasm feature disabled");
    let text = report.to_string();
    assert!(
        text.contains("wasm: not built (wasm feature disabled)"),
        "{text}"
    );
}

#[test]
fn report_keeps_pass_and_not_supported_separate() {
    let mut report = Report::default();
    report
        .runtime(super::harness::Runtime::Process)
        .record_pass();
    report
        .runtime(super::harness::Runtime::Process)
        .record_not_supported("NoProcessMemoryCap");
    let text = report.to_string();
    assert!(text.contains("process: pass 1 / not-supported 1"), "{text}");
    assert!(text.contains("NoProcessMemoryCap: 1"), "{text}");
    assert!(!report.has_failures());
}

fn assert_coverage(label: &str, all: &[&str], covered: Vec<&str>, uncovered: &[Uncovered]) {
    let mut covered = covered.into_iter().collect::<BTreeSet<_>>();
    let all = all.iter().copied().collect::<BTreeSet<_>>();
    for entry in uncovered {
        assert!(
            !entry.reason.trim().is_empty(),
            "{label} {}: empty uncovered reason",
            entry.name
        );
        assert!(
            all.contains(entry.name),
            "{label} {}: unknown uncovered entry",
            entry.name
        );
        assert!(
            covered.insert(entry.name),
            "{label} {}: both covered and uncovered",
            entry.name
        );
    }
    let missing = all.difference(&covered).copied().collect::<Vec<_>>();
    assert!(missing.is_empty(), "{label} coverage gaps: {missing:?}");
}

#[test]
fn expectation_files_are_not_empty() {
    for (name, text) in [
        ("process", PROCESS_EXPECTATIONS),
        ("wasm", WASM_EXPECTATIONS),
    ] {
        let file: ExpectationFile = parse(name, text);
        assert!(!file.rows.is_empty(), "{name}: no rows");
    }
}
