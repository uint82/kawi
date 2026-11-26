use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::GrammarError;

const SHA256_HEX_LEN: usize = 64;
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";
const HIGH_NIBBLE_SHIFT: u32 = 4;
const LOW_NIBBLE_MASK: u8 = 0x0f;

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    to_hex(Sha256::digest(bytes).as_slice())
}

pub(crate) fn is_sha256_hex(value: &str) -> bool {
    value.len() == SHA256_HEX_LEN && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn verify_sha256(path: &Path, bytes: &[u8], expected: &str) -> Result<(), GrammarError> {
    let actual = sha256_hex(bytes);
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(GrammarError::HashMismatch {
            path: path.to_path_buf(),
            expected: expected.to_owned(),
            actual,
        })
    }
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

    const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn hashes_empty_input() {
        assert_eq!(sha256_hex(b""), EMPTY_SHA256);
    }

    #[test]
    fn accepts_uppercase_hex() {
        assert!(is_sha256_hex(&EMPTY_SHA256.to_uppercase()));
    }

    #[test]
    fn rejects_the_wrong_length() {
        assert!(!is_sha256_hex("abcd"));
    }

    #[test]
    fn rejects_non_hex_characters() {
        let value = format!("{}g", &EMPTY_SHA256[..SHA256_HEX_LEN - 1]);
        assert!(!is_sha256_hex(&value));
    }

    #[test]
    fn reports_both_hashes_on_mismatch() {
        let path = Path::new("grammar.wasm");
        let expected = "0".repeat(SHA256_HEX_LEN);
        let error = verify_sha256(path, b"", &expected).expect_err("hash mismatch");
        match error {
            GrammarError::HashMismatch {
                expected: reported_expected,
                actual,
                ..
            } => {
                assert_eq!(reported_expected, expected);
                assert_eq!(actual, EMPTY_SHA256);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn formats_bytes_as_lowercase_hex() {
        assert_eq!(
            sha256_hex(b"a"),
            "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
        );
    }
}
