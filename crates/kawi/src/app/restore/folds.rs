use kawi_editor::{Editor, Fold, FoldMethod, FoldSnapshot};

use crate::session::{SessionFold, SessionFoldMethod, SessionFolds};

pub(super) fn snapshot(editor: &Editor, number: usize) -> Option<SessionFolds> {
    let folds = editor.buffer_fold_snapshot(number)?;
    if folds.is_empty() {
        return None;
    }
    let content_hash = editor.buffer_content_hash(number)?;
    Some(SessionFolds {
        method: editor.fold_method().into(),
        content_hash,
        manual: folds.manual.into_iter().map(session_fold).collect(),
        closed: folds.closed,
    })
}

pub(super) fn restore(editor: &mut Editor, number: usize, folds: &SessionFolds) {
    let Some(content_hash) = editor.buffer_content_hash(number) else {
        return;
    };
    let snapshot = restored_snapshot(folds, content_hash, editor.fold_method());
    editor.restore_buffer_folds(number, snapshot);
}

fn restored_snapshot(folds: &SessionFolds, content_hash: u64, method: FoldMethod) -> FoldSnapshot {
    let hash_matches = folds.content_hash == content_hash;
    let method_matches = FoldMethod::from(folds.method) == method;
    FoldSnapshot {
        manual: if hash_matches {
            folds.manual.iter().copied().map(fold).collect()
        } else {
            Vec::new()
        },
        closed: if method_matches && (hash_matches || folds.method == SessionFoldMethod::Indent) {
            folds.closed.clone()
        } else {
            Vec::new()
        },
    }
}

fn fold(fold: SessionFold) -> Fold {
    Fold::manual(fold.start, fold.end)
}

fn session_fold(fold: Fold) -> SessionFold {
    SessionFold {
        start: fold.start,
        end: fold.end,
    }
}
