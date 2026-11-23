use super::{ApiParameterMetadata, ApiSignatureMetadata};

const fn required(name: &'static str, ty: &'static str) -> ApiParameterMetadata {
    ApiParameterMetadata {
        name,
        ty,
        required: true,
    }
}

const fn optional(name: &'static str, ty: &'static str) -> ApiParameterMetadata {
    ApiParameterMetadata {
        name,
        ty,
        required: false,
    }
}

macro_rules! parameters {
    ($($parameter:expr),* $(,)?) => {{
        const PARAMETERS: &[$crate::api_metadata::ApiParameterMetadata] = &[$($parameter),*];
        PARAMETERS
    }};
}

mod buffers;
mod editor;
mod runtime;
mod ui;

fn signature(
    parameters: &'static [ApiParameterMetadata],
    returns: &'static str,
) -> ApiSignatureMetadata {
    ApiSignatureMetadata {
        parameters,
        returns,
    }
}

pub(super) fn method(name: &str) -> ApiSignatureMetadata {
    editor::lookup(name)
        .or_else(|| buffers::lookup(name))
        .or_else(|| ui::lookup(name))
        .or_else(|| runtime::lookup(name))
        .expect("signatures cover every host method")
}
