use serde::{Deserialize, Serialize};

use super::{
    MAX_PLUGIN_TIMERS, MAX_TIMER_DELAY_MS, MAX_TIMER_PAYLOAD_BYTES, MIN_TIMER_INTERVAL_MS,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimerAfterParams {
    pub delay_ms: u32,
    #[serde(default)]
    pub payload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimerEveryParams {
    pub interval_ms: u32,
    #[serde(default)]
    pub payload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimerCancelParams {
    pub timer: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimerInfo {
    pub id: u64,
    pub interval_ms: Option<u32>,
    pub payload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimerFired {
    pub timer: u64,
    pub payload: Option<String>,
}

pub fn validate_timer_delay(delay_ms: u32) -> Result<(), String> {
    if delay_ms > MAX_TIMER_DELAY_MS {
        return Err(format!(
            "timer delay exceeds {MAX_TIMER_DELAY_MS} ms: {delay_ms}"
        ));
    }
    Ok(())
}

pub fn validate_timer_interval(interval_ms: u32) -> Result<(), String> {
    if interval_ms < MIN_TIMER_INTERVAL_MS {
        return Err(format!(
            "timer interval must be at least {MIN_TIMER_INTERVAL_MS} ms: {interval_ms}"
        ));
    }
    if interval_ms > MAX_TIMER_DELAY_MS {
        return Err(format!(
            "timer interval exceeds {MAX_TIMER_DELAY_MS} ms: {interval_ms}"
        ));
    }
    Ok(())
}

pub fn validate_timer_payload(payload: Option<&str>) -> Result<(), String> {
    let Some(payload) = payload else {
        return Ok(());
    };
    if payload.len() > MAX_TIMER_PAYLOAD_BYTES {
        return Err(format!(
            "timer payload exceeds {MAX_TIMER_PAYLOAD_BYTES} bytes"
        ));
    }
    Ok(())
}

pub fn validate_timer_count(count: usize) -> Result<(), String> {
    if count >= MAX_PLUGIN_TIMERS {
        return Err(format!("plugin timer limit reached: {MAX_PLUGIN_TIMERS}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delays_allow_zero_and_stop_at_the_maximum() {
        assert!(validate_timer_delay(0).is_ok());
        assert!(validate_timer_delay(MAX_TIMER_DELAY_MS).is_ok());
        assert!(validate_timer_delay(MAX_TIMER_DELAY_MS + 1).is_err());
    }

    #[test]
    fn intervals_require_the_minimum_and_stop_at_the_maximum() {
        assert!(validate_timer_interval(MIN_TIMER_INTERVAL_MS - 1).is_err());
        assert!(validate_timer_interval(MIN_TIMER_INTERVAL_MS).is_ok());
        assert!(validate_timer_interval(MAX_TIMER_DELAY_MS).is_ok());
        assert!(validate_timer_interval(MAX_TIMER_DELAY_MS + 1).is_err());
    }

    #[test]
    fn payloads_are_optional_and_bounded() {
        assert!(validate_timer_payload(None).is_ok());
        assert!(validate_timer_payload(Some(&"x".repeat(MAX_TIMER_PAYLOAD_BYTES))).is_ok());
        assert!(validate_timer_payload(Some(&"x".repeat(MAX_TIMER_PAYLOAD_BYTES + 1))).is_err());
    }

    #[test]
    fn timer_counts_stop_at_the_plugin_limit() {
        assert!(validate_timer_count(0).is_ok());
        assert!(validate_timer_count(MAX_PLUGIN_TIMERS - 1).is_ok());
        assert!(validate_timer_count(MAX_PLUGIN_TIMERS).is_err());
    }
}
