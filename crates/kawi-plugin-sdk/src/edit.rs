use crate::types::{Edit, Error, State};
use crate::{editor_read, editor_write, ErrorKind};

const MAX_STALE_RETRIES: usize = 2;

pub fn apply_edit_with_retry(
    buffer: u64,
    revision: u64,
    edits: Vec<Edit>,
    mut rebuild: impl FnMut(&State) -> Result<Vec<Edit>, Error>,
) -> Result<u64, Error> {
    let mut current_revision = revision;
    let mut current_edits = edits;
    let mut retries = 0;
    loop {
        match editor_write::apply_edit(buffer, current_revision, &current_edits) {
            Ok(revision) => return Ok(revision),
            Err(error)
                if crate::error_kind(&error) == ErrorKind::StaleRevision
                    && retries < MAX_STALE_RETRIES =>
            {
                retries += 1;
                let state = editor_read::get_state(true)?;
                if state.buffer != buffer {
                    return Err(error);
                }
                current_revision = state.revision;
                current_edits = rebuild(&state)?;
                if current_edits.is_empty() {
                    return Ok(state.revision);
                }
            }
            Err(error) => return Err(error),
        }
    }
}
