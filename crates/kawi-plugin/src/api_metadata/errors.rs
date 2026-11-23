use super::ApiErrorMetadata;
use crate::protocol::{
    BUFFER_MODIFIED, BUFFER_NOT_ACTIVE, BUFFER_NOT_FOUND, BUFFER_NOT_OWNED, CAPABILITY_DENIED,
    INTERNAL_ERROR, INVALID_PARAMS, INVALID_POSITION, INVALID_REQUEST, JOB_CLOSED, JOB_LIMIT,
    JOB_NOT_FOUND, JOB_QUEUE_FULL, LAST_WINDOW, METHOD_NOT_FOUND, MODE_NOT_SUPPORTED,
    OPTION_NOT_FOUND, OVERLAY_NOT_FOUND, PANEL_NOT_FOUND, PARSE_ERROR, PLUGIN_ERROR,
    PLUGIN_UNAVAILABLE, REQUEST_CANCELED, STALE_REVISION, TASK_LIMIT, TASK_NOT_FOUND,
    TASK_START_FAILED, UI_BUSY, WINDOW_NOT_FOUND,
};

pub(super) const ERROR_TYPES: &[ApiErrorMetadata] = &[
    ApiErrorMetadata {
        name: "PARSE_ERROR",
        code: PARSE_ERROR,
        description: "Parse error",
    },
    ApiErrorMetadata {
        name: "INVALID_REQUEST",
        code: INVALID_REQUEST,
        description: "Invalid request",
    },
    ApiErrorMetadata {
        name: "METHOD_NOT_FOUND",
        code: METHOD_NOT_FOUND,
        description: "Method not found or not declared",
    },
    ApiErrorMetadata {
        name: "INVALID_PARAMS",
        code: INVALID_PARAMS,
        description: "Invalid params",
    },
    ApiErrorMetadata {
        name: "INTERNAL_ERROR",
        code: INTERNAL_ERROR,
        description: "Internal error",
    },
    ApiErrorMetadata {
        name: "PLUGIN_ERROR",
        code: PLUGIN_ERROR,
        description: "Plugin command error",
    },
    ApiErrorMetadata {
        name: "STALE_REVISION",
        code: STALE_REVISION,
        description: "Stale buffer revision",
    },
    ApiErrorMetadata {
        name: "INVALID_POSITION",
        code: INVALID_POSITION,
        description: "Invalid position",
    },
    ApiErrorMetadata {
        name: "CAPABILITY_DENIED",
        code: CAPABILITY_DENIED,
        description: "Capability denied",
    },
    ApiErrorMetadata {
        name: "REQUEST_CANCELED",
        code: REQUEST_CANCELED,
        description: "Request canceled",
    },
    ApiErrorMetadata {
        name: "PLUGIN_UNAVAILABLE",
        code: PLUGIN_UNAVAILABLE,
        description: "Plugin unavailable",
    },
    ApiErrorMetadata {
        name: "UI_BUSY",
        code: UI_BUSY,
        description: "Interactive UI is busy",
    },
    ApiErrorMetadata {
        name: "JOB_NOT_FOUND",
        code: JOB_NOT_FOUND,
        description: "Job not found",
    },
    ApiErrorMetadata {
        name: "JOB_LIMIT",
        code: JOB_LIMIT,
        description: "Too many running jobs",
    },
    ApiErrorMetadata {
        name: "JOB_CLOSED",
        code: JOB_CLOSED,
        description: "Job or job stdin is closed",
    },
    ApiErrorMetadata {
        name: "JOB_QUEUE_FULL",
        code: JOB_QUEUE_FULL,
        description: "Job output or input queue is full",
    },
    ApiErrorMetadata {
        name: "PANEL_NOT_FOUND",
        code: PANEL_NOT_FOUND,
        description: "Panel not found or owned by another plugin",
    },
    ApiErrorMetadata {
        name: "OVERLAY_NOT_FOUND",
        code: OVERLAY_NOT_FOUND,
        description: "Overlay not found or owned by another plugin",
    },
    ApiErrorMetadata {
        name: "TASK_NOT_FOUND",
        code: TASK_NOT_FOUND,
        description: "Task not found",
    },
    ApiErrorMetadata {
        name: "TASK_LIMIT",
        code: TASK_LIMIT,
        description: "Too many running tasks",
    },
    ApiErrorMetadata {
        name: "TASK_START_FAILED",
        code: TASK_START_FAILED,
        description: "Task failed to start",
    },
    ApiErrorMetadata {
        name: "BUFFER_NOT_FOUND",
        code: BUFFER_NOT_FOUND,
        description: "Buffer not found",
    },
    ApiErrorMetadata {
        name: "BUFFER_NOT_ACTIVE",
        code: BUFFER_NOT_ACTIVE,
        description: "Buffer not active",
    },
    ApiErrorMetadata {
        name: "WINDOW_NOT_FOUND",
        code: WINDOW_NOT_FOUND,
        description: "Window not found",
    },
    ApiErrorMetadata {
        name: "LAST_WINDOW",
        code: LAST_WINDOW,
        description: "Cannot close the last window",
    },
    ApiErrorMetadata {
        name: "OPTION_NOT_FOUND",
        code: OPTION_NOT_FOUND,
        description: "Option not found",
    },
    ApiErrorMetadata {
        name: "MODE_NOT_SUPPORTED",
        code: MODE_NOT_SUPPORTED,
        description: "Mode not supported",
    },
    ApiErrorMetadata {
        name: "BUFFER_MODIFIED",
        code: BUFFER_MODIFIED,
        description: "Buffer has unsaved changes",
    },
    ApiErrorMetadata {
        name: "BUFFER_NOT_OWNED",
        code: BUFFER_NOT_OWNED,
        description: "Buffer is not owned by the plugin",
    },
];
