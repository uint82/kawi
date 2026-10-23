use sha2::{Digest, Sha256};
use std::path::Path;

pub(crate) fn digest_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut key = String::with_capacity(64);
    for byte in digest {
        key.push(HEX[(byte >> 4) as usize] as char);
        key.push(HEX[(byte & 15) as usize] as char);
    }
    key
}

pub(crate) fn key(path: &Path) -> String {
    digest_hex(path.as_os_str().as_encoded_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digests_are_stable_and_distinct() {
        assert_eq!(digest_hex(b"alpha"), digest_hex(b"alpha"));
        assert_ne!(digest_hex(b"alpha"), digest_hex(b"beta"));
        assert_eq!(digest_hex(b"alpha").len(), 64);
    }

    #[test]
    fn path_keys_are_stable_and_distinct() {
        assert_eq!(key(Path::new("/tmp/alpha")), key(Path::new("/tmp/alpha")));
        assert_ne!(key(Path::new("/tmp/alpha")), key(Path::new("/tmp/beta")));
    }
}
