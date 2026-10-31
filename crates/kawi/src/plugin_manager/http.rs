use std::io::Read;
use std::time::Duration;

use serde::Deserialize;

const USER_AGENT: &str = "kawi-plugin-manager";
const API_VERSION: &str = "2022-11-28";
const TIMEOUT_SECONDS: u64 = 30;
const TOKEN_ENV: &str = "GITHUB_TOKEN";

pub(crate) struct HttpClient {
    agent: ureq::Agent,
    token: Option<String>,
}

impl HttpClient {
    pub(crate) fn new() -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(TIMEOUT_SECONDS))
            .build();
        Self {
            agent,
            token: std::env::var(TOKEN_ENV)
                .ok()
                .filter(|token| !token.is_empty()),
        }
    }

    pub(crate) fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        limit: usize,
    ) -> Result<T, String> {
        let text = self.get_text(url, limit)?;
        serde_json::from_str(&text)
            .map_err(|error| format!("cannot parse GitHub response: {error}"))
    }

    pub(crate) fn get_optional_json<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        limit: usize,
    ) -> Result<Option<T>, String> {
        match self.request(url) {
            Ok(response) => {
                let text = read_text(response, limit)?;
                serde_json::from_str(&text)
                    .map(Some)
                    .map_err(|error| format!("cannot parse GitHub response: {error}"))
            }
            Err(HttpError::NotFound) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    pub(crate) fn get_text(&self, url: &str, limit: usize) -> Result<String, String> {
        self.request(url)
            .map_err(|error| error.to_string())
            .and_then(|response| read_text(response, limit))
    }

    pub(crate) fn get_bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>, String> {
        self.request(url)
            .map_err(|error| error.to_string())
            .and_then(|response| read_bytes(response, limit))
    }

    fn request(&self, url: &str) -> Result<ureq::Response, HttpError> {
        let mut request = self
            .agent
            .get(url)
            .set("User-Agent", USER_AGENT)
            .set("Accept", "application/vnd.github+json")
            .set("X-GitHub-Api-Version", API_VERSION);
        if let Some(token) = &self.token {
            request = request.set("Authorization", &format!("Bearer {token}"));
        }
        request.call().map_err(HttpError::from)
    }
}

enum HttpError {
    NotFound,
    Status(u16, String),
    Transport(String),
}

impl From<ureq::Error> for HttpError {
    fn from(error: ureq::Error) -> Self {
        match error {
            ureq::Error::Status(404, _) => Self::NotFound,
            ureq::Error::Status(status, response) => {
                let message = response
                    .into_string()
                    .unwrap_or_else(|_| "GitHub request failed".to_string());
                Self::Status(status, message)
            }
            ureq::Error::Transport(error) => Self::Transport(error.to_string()),
        }
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("GitHub resource not found"),
            Self::Status(status, message) => {
                write!(
                    formatter,
                    "GitHub request failed with status {status}: {message}"
                )
            }
            Self::Transport(message) => write!(formatter, "GitHub request failed: {message}"),
        }
    }
}

fn read_text(response: ureq::Response, limit: usize) -> Result<String, String> {
    let bytes = read_bytes(response, limit)?;
    String::from_utf8(bytes).map_err(|error| format!("GitHub response is not UTF-8: {error}"))
}

fn read_bytes(response: ureq::Response, limit: usize) -> Result<Vec<u8>, String> {
    if response
        .header("Content-Length")
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|length| length > limit)
    {
        return Err(format!("download exceeds {limit} bytes"));
    }
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read download: {error}"))?;
    if bytes.len() > limit {
        return Err(format!("download exceeds {limit} bytes"));
    }
    Ok(bytes)
}

pub(crate) fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::encode_path_segment;

    #[test]
    fn encodes_reference_path_segments() {
        assert_eq!(encode_path_segment("release/v1"), "release%2Fv1");
        assert_eq!(encode_path_segment("v1.0.0"), "v1.0.0");
    }
}
