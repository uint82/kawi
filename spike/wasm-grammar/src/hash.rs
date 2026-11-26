use std::error::Error;
use std::fmt;

use sha2::{Digest, Sha256};

const SHA256_HEX_LEN: usize = 64;
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";
const HIGH_NIBBLE_SHIFT: u32 = 4;
const LOW_NIBBLE_MASK: u8 = 0x0f;

#[derive(Debug, PartialEq, Eq)]
pub enum HashError {
    InvalidLength { actual: usize },
    InvalidHex { index: usize },
    Mismatch { expected: String, actual: String },
}

impl fmt::Display for HashError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength { actual } => {
                write!(
                    formatter,
                    "expected {SHA256_HEX_LEN} hex characters, got {actual}"
                )
            }
            Self::InvalidHex { index } => {
                write!(formatter, "invalid hex character at index {index}")
            }
            Self::Mismatch { expected, actual } => {
                write!(formatter, "expected {expected}, got {actual}")
            }
        }
    }
}

impl Error for HashError {}

pub fn sha256_hex(bytes: &[u8]) -> String {
    to_hex(Sha256::digest(bytes).as_slice())
}

pub fn verify_sha256(bytes: &[u8], expected: &str) -> Result<(), HashError> {
    validate_sha256_hex(expected)?;
    let actual = sha256_hex(bytes);
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(HashError::Mismatch {
            expected: expected.to_owned(),
            actual,
        })
    }
}

fn validate_sha256_hex(value: &str) -> Result<(), HashError> {
    if value.len() != SHA256_HEX_LEN {
        return Err(HashError::InvalidLength {
            actual: value.len(),
        });
    }
    if let Some(index) = value.bytes().position(|byte| !byte.is_ascii_hexdigit()) {
        return Err(HashError::InvalidHex { index });
    }
    Ok(())
}

fn to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(HEX_DIGITS[(byte >> HIGH_NIBBLE_SHIFT) as usize] as char);
        output.push(HEX_DIGITS[(byte & LOW_NIBBLE_MASK) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY_SHA256: &str =
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn hashes_empty_input() {
        assert_eq!(sha256_hex(b""), EMPTY_SHA256);
    }

    #[test]
    fn accepts_uppercase_hex() {
        assert_eq!(verify_sha256(b"", &EMPTY_SHA256.to_uppercase()), Ok(()));
    }

    #[test]
    fn rejects_the_wrong_length() {
        assert_eq!(
            verify_sha256(b"", "abcd"),
            Err(HashError::InvalidLength { actual: 4 })
        );
    }

    #[test]
    fn rejects_non_hex_characters() {
        let expected = format!("{}g", &EMPTY_SHA256[..SHA256_HEX_LEN - 1]);
        assert_eq!(
            verify_sha256(b"", &expected),
            Err(HashError::InvalidHex {
                index: SHA256_HEX_LEN - 1
            })
        );
    }

    #[test]
    fn reports_both_hashes_on_mismatch() {
        let expected = "0".repeat(SHA256_HEX_LEN);
        assert_eq!(
            verify_sha256(b"", &expected),
            Err(HashError::Mismatch {
                expected,
                actual: EMPTY_SHA256.to_owned(),
            })
        );
    }

    #[test]
    fn formats_bytes_as_lowercase_hex() {
        assert_eq!(
            sha256_hex(b"a"),
            "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
        );
    }
}
