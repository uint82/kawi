const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

pub(crate) struct ContentHasher {
    hash: u64,
}

impl ContentHasher {
    pub(crate) fn new() -> Self {
        Self { hash: OFFSET_BASIS }
    }

    pub(crate) fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.hash ^= u64::from(*byte);
            self.hash = self.hash.wrapping_mul(PRIME);
        }
    }

    pub(crate) fn finish(self) -> u64 {
        self.hash
    }
}

pub fn content_hash(bytes: &[u8]) -> u64 {
    let mut hasher = ContentHasher::new();
    hasher.write(bytes);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_changes_with_content() {
        assert_ne!(content_hash(b"a"), content_hash(b"b"));
    }

    #[test]
    fn streaming_matches_a_single_write() {
        let mut hasher = ContentHasher::new();
        hasher.write(b"ab");
        hasher.write(b"c");
        assert_eq!(hasher.finish(), content_hash(b"abc"));
    }
}
