use super::bindings::kawi::plugin::types::Error;
use super::bindings::kawi::plugin::ui_panel::{self, OpenParams, OpenResult, UpdateParams};
use super::convert::{decoration_style_to_protocol, error_from_rpc};
use super::state::{decode_json, serialize_params, WasmState};
use crate::protocol::{
    PanelLine, PanelOpenParams, PanelOpenResult, PanelParams, PanelUpdateParams,
};

impl ui_panel::Host for WasmState {
    fn open(&mut self, params: OpenParams) -> Result<OpenResult, Error> {
        let params = open_params(params);
        let value = self
            .bridge
            .request("ui.panel_open", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        let result: PanelOpenResult = decode_json(value)?;
        Ok(OpenResult {
            panel: result.panel,
            width: result.width,
            height: result.height,
        })
    }

    fn update(&mut self, params: UpdateParams) -> Result<(), Error> {
        let params = update_params(params);
        self.bridge
            .request("ui.panel_update", serialize_params(params)?)
            .map(|_| ())
            .map_err(error_from_rpc)
    }

    fn focus(&mut self, panel: u64) -> Result<(), Error> {
        let params = PanelParams { panel };
        self.bridge
            .request("ui.panel_focus", serialize_params(params)?)
            .map(|_| ())
            .map_err(error_from_rpc)
    }

    fn close(&mut self, panel: u64) -> Result<(), Error> {
        let params = PanelParams { panel };
        self.bridge
            .request("ui.panel_close", serialize_params(params)?)
            .map(|_| ())
            .map_err(error_from_rpc)
    }
}

fn open_params(params: OpenParams) -> PanelOpenParams {
    PanelOpenParams {
        title: params.title,
        status: params.status,
        height: params.height,
        focus: params.focus,
        lines: panel_lines(params.lines),
    }
}

fn update_params(params: UpdateParams) -> PanelUpdateParams {
    PanelUpdateParams {
        panel: params.panel,
        title: params.title,
        status: params.status,
        height: params.height,
        lines: params.lines.map(panel_lines),
    }
}

fn panel_lines(lines: Vec<ui_panel::PanelLine>) -> Vec<PanelLine> {
    lines
        .into_iter()
        .map(|line| PanelLine {
            text: line.text,
            style: line.style.map(decoration_style_to_protocol),
        })
        .collect()
}
