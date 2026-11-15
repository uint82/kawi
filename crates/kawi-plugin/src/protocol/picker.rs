use serde::{Deserialize, Serialize};

use super::{RpcError, INVALID_PARAMS};

pub const MAX_PICKER_ITEMS: usize = 256;
pub const MAX_PICKER_ID_BYTES: usize = 256;
pub const MAX_PICKER_LABEL_BYTES: usize = 512;
pub const MAX_PICKER_DETAIL_BYTES: usize = 512;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PickerItem {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub deprecated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PickerParams {
    pub title: String,
    pub items: Vec<PickerItem>,
    #[serde(default)]
    pub selected: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PickerOutcome {
    Selected(String),
    Canceled,
}

pub fn validate_picker(params: &PickerParams) -> Result<(), RpcError> {
    if params.items.is_empty() {
        return Err(RpcError::new(
            INVALID_PARAMS,
            "picker items must not be empty",
        ));
    }
    if params.items.len() > MAX_PICKER_ITEMS {
        return Err(RpcError::new(
            INVALID_PARAMS,
            format!("at most {MAX_PICKER_ITEMS} picker items are accepted"),
        ));
    }
    if let Some(selected) = params.selected {
        if selected as usize >= params.items.len() {
            return Err(RpcError::new(INVALID_PARAMS, "selected is out of range"));
        }
    }
    let mut ids = std::collections::BTreeSet::new();
    for item in &params.items {
        validate_text("picker item id", &item.id, MAX_PICKER_ID_BYTES)?;
        validate_text("picker item label", &item.label, MAX_PICKER_LABEL_BYTES)?;
        if item.id.is_empty() {
            return Err(RpcError::new(
                INVALID_PARAMS,
                "picker item id must not be empty",
            ));
        }
        if item.label.is_empty() {
            return Err(RpcError::new(
                INVALID_PARAMS,
                "picker item label must not be empty",
            ));
        }
        if let Some(detail) = item.detail.as_deref() {
            validate_text("picker item detail", detail, MAX_PICKER_DETAIL_BYTES)?;
        }
        if !ids.insert(item.id.as_str()) {
            return Err(RpcError::new(
                INVALID_PARAMS,
                "picker item ids must be unique",
            ));
        }
    }
    Ok(())
}

fn validate_text(label: &str, text: &str, limit: usize) -> Result<(), RpcError> {
    if text.len() > limit {
        return Err(RpcError::new(
            INVALID_PARAMS,
            format!("{label} exceeds {limit} bytes"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn item(id: &str, label: &str) -> PickerItem {
        PickerItem {
            id: id.to_string(),
            label: label.to_string(),
            detail: None,
            deprecated: false,
        }
    }

    #[test]
    fn picker_defaults_selected_to_the_first_item() {
        let params: PickerParams = serde_json::from_value(json!({
            "title": "Commands",
            "items": [{"id": "a", "label": "Alpha"}],
        }))
        .unwrap();
        assert_eq!(params.selected, None);
        assert!(validate_picker(&params).is_ok());
    }

    #[test]
    fn rejects_duplicate_ids() {
        let params = PickerParams {
            title: "Commands".to_string(),
            items: vec![item("a", "Alpha"), item("a", "Beta")],
            selected: None,
        };
        assert!(validate_picker(&params).is_err());
    }

    #[test]
    fn rejects_out_of_range_selection() {
        let params = PickerParams {
            title: "Commands".to_string(),
            items: vec![item("a", "Alpha")],
            selected: Some(1),
        };
        assert!(validate_picker(&params).is_err());
    }
}
