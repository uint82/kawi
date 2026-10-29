use serde_json::{json, Value};

use kawi_plugin::{resolver_call_count, PluginError, CAPABILITY_DENIED};

use super::super::super::harness::{ConformanceHost, Runtime};
use super::super::super::scenario::{resolver_script, NetworkTarget, ResolverScript, Scenario};

pub(crate) fn run_network_scripted(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    assert_eq!(runtime, Runtime::Wasm);
    let script = resolver_script(scenario.id).expect("scripted network scenario");
    match scenario.id {
        "wasm.net.rebind_private" => run_rebind_private(host, script),
        "wasm.net.rebind_public" => run_rebind_public(host, script),
        "wasm.net.mixed_answers" => run_mixed_answers(host, script),
        "wasm.net.all_private" => run_all_private(host, script),
        _ => Err(PluginError::Protocol(format!(
            "unexpected scripted network scenario: {}",
            scenario.id
        ))),
    }
}

fn run_rebind_private(
    host: &ConformanceHost,
    script: ResolverScript,
) -> Result<Value, PluginError> {
    let pinned = script.answers[0][0];
    let rebound = script.answers[1][0];
    expect_lookup(host, pinned)?;
    expect_lookup(host, pinned)?;
    expect_call_count(script, 1)?;
    expect_denied(host, rebound)?;
    host.restart();
    let result = lookup(host);
    assert_denied(&result)?;
    expect_call_count(script, 2)?;
    result
}

fn run_rebind_public(host: &ConformanceHost, script: ResolverScript) -> Result<Value, PluginError> {
    let pinned = script.answers[0][0];
    let rebound = script.answers[1][0];
    expect_lookup(host, pinned)?;
    expect_lookup(host, pinned)?;
    expect_call_count(script, 1)?;
    expect_denied(host, rebound)?;
    host.restart();
    expect_lookup(host, rebound)?;
    expect_call_count(script, 2)?;
    let result = connect(host, pinned);
    assert_denied(&result)?;
    result
}

fn run_mixed_answers(host: &ConformanceHost, script: ResolverScript) -> Result<Value, PluginError> {
    let answer = script.answers[0];
    expect_lookup(host, answer[0])?;
    expect_lookup(host, answer[0])?;
    expect_call_count(script, 1)?;
    let mut result = None;
    for (index, target) in answer.iter().skip(1).enumerate() {
        if index > 0 {
            host.restart();
        }
        let denied = connect(host, target);
        assert_denied(&denied)?;
        result = Some(denied);
    }
    expect_call_count(script, script.answers.len())?;
    result.ok_or_else(|| PluginError::Protocol("mixed answer has no denied targets".to_string()))?
}

fn run_all_private(host: &ConformanceHost, script: ResolverScript) -> Result<Value, PluginError> {
    let result = lookup(host);
    assert_denied(&result)?;
    expect_call_count(script, 1)?;
    result
}

fn lookup(host: &ConformanceHost) -> Result<Value, PluginError> {
    host.invoke(
        "resolve-list",
        json!([host.network_target(NetworkTarget::Hostname)]),
    )
}

fn connect(host: &ConformanceHost, target: &str) -> Result<Value, PluginError> {
    host.invoke("connect", json!([target]))
}

fn expect_lookup(host: &ConformanceHost, expected: &str) -> Result<(), PluginError> {
    let value = lookup(host)?;
    let actual = value.as_str().unwrap_or_default();
    if actual == expected {
        Ok(())
    } else {
        Err(PluginError::Protocol(format!(
            "guest lookup returned {actual}, expected {expected}"
        )))
    }
}

fn expect_denied(host: &ConformanceHost, target: &str) -> Result<(), PluginError> {
    let result = connect(host, target);
    assert_denied(&result)
}

fn assert_denied(result: &Result<Value, PluginError>) -> Result<(), PluginError> {
    match result {
        Err(PluginError::Rpc(error)) if error.code == CAPABILITY_DENIED => Ok(()),
        Err(error) => Err(PluginError::Protocol(format!(
            "expected a network denial, observed {error}"
        ))),
        Ok(value) => Err(PluginError::Protocol(format!(
            "expected a network denial, observed {value}"
        ))),
    }
}

fn expect_call_count(script: ResolverScript, expected: usize) -> Result<(), PluginError> {
    let actual = resolver_call_count(script.host);
    if actual == expected {
        Ok(())
    } else {
        Err(PluginError::Protocol(format!(
            "resolver called {actual} times, expected {expected}"
        )))
    }
}
