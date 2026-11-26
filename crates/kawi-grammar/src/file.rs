use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::error::GrammarError;

pub(crate) fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, GrammarError> {
    let file = File::open(path).map_err(|source| GrammarError::io(path, source))?;
    let metadata = file
        .metadata()
        .map_err(|source| GrammarError::io(path, source))?;
    if metadata.len() > limit {
        return Err(GrammarError::TooLarge {
            path: path.to_path_buf(),
            size: metadata.len(),
            limit,
        });
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| GrammarError::io(path, source))?;
    if bytes.len() as u64 > limit {
        return Err(GrammarError::TooLarge {
            path: path.to_path_buf(),
            size: bytes.len() as u64,
            limit,
        });
    }
    Ok(bytes)
}
