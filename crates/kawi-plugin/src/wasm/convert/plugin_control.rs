use serde_json::Value;

use crate::protocol::{
    BufferInfo, BufferState, OptionInfo, RpcError, Selection, WindowInfo, INVALID_PARAMS,
};

use super::super::bindings::kawi::plugin::types as wit;

pub fn buffer_infos_from_json(value: Value) -> Result<Vec<wit::BufferInfo>, RpcError> {
    let items: Vec<BufferInfo> = serde_json::from_value(value)
        .map_err(|error| RpcError::new(INVALID_PARAMS, error.to_string()))?;
    items.into_iter().map(buffer_info_from_protocol).collect()
}

pub fn buffer_state_from_json(value: Value) -> Result<wit::BufferState, RpcError> {
    let state: BufferState = serde_json::from_value(value)
        .map_err(|error| RpcError::new(INVALID_PARAMS, error.to_string()))?;
    Ok(wit::BufferState {
        buffer: state.buffer,
        revision: state.revision,
        path: state.path,
        language: state.language,
        modified: state.modified,
        text: state.text,
    })
}

pub fn window_infos_from_json(value: Value) -> Result<Vec<wit::WindowInfo>, RpcError> {
    let items: Vec<WindowInfo> = serde_json::from_value(value)
        .map_err(|error| RpcError::new(INVALID_PARAMS, error.to_string()))?;
    items.into_iter().map(window_info_from_protocol).collect()
}

pub fn window_info_from_json(value: Value) -> Result<wit::WindowInfo, RpcError> {
    let info: WindowInfo = serde_json::from_value(value)
        .map_err(|error| RpcError::new(INVALID_PARAMS, error.to_string()))?;
    window_info_from_protocol(info)
}

pub fn option_infos_from_json(value: Value) -> Result<Vec<wit::OptionInfo>, RpcError> {
    let items: Vec<OptionInfo> = serde_json::from_value(value)
        .map_err(|error| RpcError::new(INVALID_PARAMS, error.to_string()))?;
    items.into_iter().map(option_info_from_protocol).collect()
}

pub fn option_info_from_json(value: Value) -> Result<wit::OptionInfo, RpcError> {
    let info: OptionInfo = serde_json::from_value(value)
        .map_err(|error| RpcError::new(INVALID_PARAMS, error.to_string()))?;
    option_info_from_protocol(info)
}

pub fn selection_from_protocol(selection: Selection) -> Result<wit::Selection, RpcError> {
    Ok(wit::Selection {
        start: super::position_from_protocol(selection.start)?,
        end: super::position_from_protocol(selection.end)?,
    })
}

pub fn selection_to_protocol(selection: wit::Selection) -> Result<Selection, RpcError> {
    Ok(Selection {
        start: super::position_to_protocol(selection.start)?,
        end: super::position_to_protocol(selection.end)?,
    })
}

pub fn split_direction_name(direction: wit::SplitDirection) -> &'static str {
    match direction {
        wit::SplitDirection::Horizontal => "horizontal",
        wit::SplitDirection::Vertical => "vertical",
    }
}

fn buffer_info_from_protocol(info: BufferInfo) -> Result<wit::BufferInfo, RpcError> {
    Ok(wit::BufferInfo {
        buffer: info.buffer,
        revision: info.revision,
        path: info.path,
        language: info.language,
        modified: info.modified,
    })
}

fn window_info_from_protocol(info: WindowInfo) -> Result<wit::WindowInfo, RpcError> {
    Ok(wit::WindowInfo {
        window: info.window,
        buffer: info.buffer,
        active: info.active,
        width: info.width,
        height: info.height,
    })
}

fn option_info_from_protocol(info: OptionInfo) -> Result<wit::OptionInfo, RpcError> {
    Ok(wit::OptionInfo {
        name: info.name,
        aliases: info.aliases,
        description: info.description,
        kind: option_kind_from_name(&info.kind)?,
        scope: option_scope_from_name(&info.scope)?,
        section: info.section,
        default: info.default,
        value: info.value,
        values: info.values,
    })
}

fn option_kind_from_name(name: &str) -> Result<wit::OptionKind, RpcError> {
    match name {
        "boolean" => Ok(wit::OptionKind::Boolean),
        "integer" => Ok(wit::OptionKind::Integer),
        "text" => Ok(wit::OptionKind::Text),
        "enumeration" => Ok(wit::OptionKind::Enumeration),
        "list" | "list-values" => Ok(wit::OptionKind::ListValues),
        other => Err(RpcError::new(
            INVALID_PARAMS,
            format!("unknown option kind: {other}"),
        )),
    }
}

fn option_scope_from_name(name: &str) -> Result<wit::OptionScope, RpcError> {
    match name {
        "global" => Ok(wit::OptionScope::Global),
        "buffer" => Ok(wit::OptionScope::Buffer),
        "window" => Ok(wit::OptionScope::Window),
        other => Err(RpcError::new(
            INVALID_PARAMS,
            format!("unknown option scope: {other}"),
        )),
    }
}
