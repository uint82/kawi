use kawi_editor::{Fold, FoldMethod, FoldSnapshot};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StoredFoldMethod {
    Manual,
    Indent,
}

impl From<FoldMethod> for StoredFoldMethod {
    fn from(method: FoldMethod) -> Self {
        match method {
            FoldMethod::Manual => Self::Manual,
            FoldMethod::Indent => Self::Indent,
        }
    }
}

impl From<StoredFoldMethod> for FoldMethod {
    fn from(method: StoredFoldMethod) -> Self {
        match method {
            StoredFoldMethod::Manual => Self::Manual,
            StoredFoldMethod::Indent => Self::Indent,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StoredFold {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FoldView {
    pub method: StoredFoldMethod,
    pub content_hash: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manual: Vec<StoredFold>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub closed: Vec<usize>,
}

impl FoldView {
    pub(crate) fn from_snapshot(
        snapshot: FoldSnapshot,
        method: FoldMethod,
        content_hash: u64,
    ) -> Self {
        Self {
            method: method.into(),
            content_hash,
            manual: snapshot.manual.into_iter().map(stored_fold).collect(),
            closed: snapshot.closed,
        }
    }

    pub(crate) fn restore_snapshot(&self, content_hash: u64, method: FoldMethod) -> FoldSnapshot {
        let hash_matches = self.content_hash == content_hash;
        let method_matches = FoldMethod::from(self.method) == method;
        FoldSnapshot {
            manual: if hash_matches {
                self.manual.iter().copied().map(editor_fold).collect()
            } else {
                Vec::new()
            },
            closed: if method_matches && (hash_matches || self.method == StoredFoldMethod::Indent) {
                self.closed.clone()
            } else {
                Vec::new()
            },
        }
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.manual.iter().any(|fold| fold.start >= fold.end) {
            return Err("fold range is empty or reversed".to_string());
        }
        Ok(())
    }
}

fn stored_fold(fold: Fold) -> StoredFold {
    StoredFold {
        start: fold.start,
        end: fold.end,
    }
}

fn editor_fold(fold: StoredFold) -> Fold {
    Fold::manual(fold.start, fold.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> FoldView {
        FoldView {
            method: StoredFoldMethod::Manual,
            content_hash: 7,
            manual: vec![StoredFold { start: 1, end: 3 }],
            closed: vec![1],
        }
    }

    #[test]
    fn snapshot_round_trips_through_a_view() {
        let snapshot = FoldSnapshot {
            manual: vec![Fold::manual(1, 3)],
            closed: vec![1],
        };
        let view = FoldView::from_snapshot(snapshot.clone(), FoldMethod::Manual, 7);
        assert_eq!(view.restore_snapshot(7, FoldMethod::Manual), snapshot);
    }

    #[test]
    fn changed_content_drops_manual_folds() {
        let view = view();
        let restored = view.restore_snapshot(8, FoldMethod::Manual);
        assert!(restored.manual.is_empty());
        assert!(restored.closed.is_empty());
    }

    #[test]
    fn changed_content_keeps_indent_closed_starts() {
        let mut view = view();
        view.method = StoredFoldMethod::Indent;
        let restored = view.restore_snapshot(8, FoldMethod::Indent);
        assert!(restored.manual.is_empty());
        assert_eq!(restored.closed, vec![1]);
    }

    #[test]
    fn another_method_drops_closed_starts() {
        let view = view();
        let restored = view.restore_snapshot(7, FoldMethod::Indent);
        assert_eq!(restored.closed, Vec::<usize>::new());
    }

    #[test]
    fn empty_and_reversed_ranges_are_rejected() {
        let mut view = view();
        view.manual = vec![StoredFold { start: 3, end: 3 }];
        assert_eq!(
            view.validate(),
            Err("fold range is empty or reversed".to_string())
        );
    }
}
