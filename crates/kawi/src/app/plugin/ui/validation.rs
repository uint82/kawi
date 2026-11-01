use kawi_plugin::RpcError;

use super::super::text::sanitize;

pub(super) const MAX_TITLE_BYTES: usize = 256;
pub(super) const MAX_MESSAGE_BYTES: usize = 1024;
pub(super) const MAX_PLACEHOLDER_BYTES: usize = 256;
pub(super) const MAX_INPUT_BYTES: usize = 4096;
pub(super) const DEFAULT_INPUT_BYTES: usize = 1024;
pub(super) const MAX_SELECT_OPTIONS: usize = 256;
pub(super) const MAX_SELECT_OPTION_BYTES: usize = 512;
pub(super) const MAX_PROGRESS_MESSAGE_BYTES: usize = 512;

pub(super) fn required_text(
    value: &str,
    field: &str,
    max_bytes: usize,
) -> Result<String, RpcError> {
    let text = sanitize(value);
    if text.trim().is_empty() {
        return Err(RpcError::invalid_params(format!(
            "{field} must not be empty"
        )));
    }
    if text.len() > max_bytes {
        return Err(RpcError::invalid_params(format!(
            "{field} must not exceed {max_bytes} bytes"
        )));
    }
    Ok(text)
}

pub(super) fn optional_text(
    value: Option<&str>,
    field: &str,
    max_bytes: usize,
) -> Result<Option<String>, RpcError> {
    value
        .map(sanitize)
        .map(|text| {
            if text.len() > max_bytes {
                Err(RpcError::invalid_params(format!(
                    "{field} must not exceed {max_bytes} bytes"
                )))
            } else {
                Ok(text)
            }
        })
        .transpose()
}

pub(super) fn validate_fraction(fraction: Option<f32>) -> Result<(), RpcError> {
    if fraction.is_some_and(|fraction| !fraction.is_finite() || !(0.0..=1.0).contains(&fraction)) {
        return Err(RpcError::invalid_params(
            "fraction must be between 0.0 and 1.0",
        ));
    }
    Ok(())
}
