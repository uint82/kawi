use super::bindings::kawi::plugin::types::Error;
use super::bindings::kawi::plugin::ui_picker::{
    self, PickerItem, PickerOutcome, PickerParams as WitPickerParams,
};
use super::convert::error_from_rpc;
use super::state::{decode_json, serialize_params, WasmState};
use crate::protocol::{
    PickerItem as ProtocolPickerItem, PickerOutcome as ProtocolPickerOutcome,
    PickerParams as ProtocolPickerParams,
};

impl ui_picker::Host for WasmState {
    fn pick(&mut self, params: WitPickerParams) -> Result<PickerOutcome, Error> {
        let params = ProtocolPickerParams {
            title: params.title,
            items: params.items.into_iter().map(item_to_protocol).collect(),
            selected: params.selected,
        };
        let value = self
            .bridge
            .request_interactive("ui.picker", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        let outcome: ProtocolPickerOutcome = decode_json(value)?;
        Ok(outcome_to_wit(outcome))
    }
}

fn item_to_protocol(item: PickerItem) -> ProtocolPickerItem {
    ProtocolPickerItem {
        id: item.id,
        label: item.label,
        detail: item.detail,
        deprecated: item.deprecated,
    }
}

fn outcome_to_wit(outcome: ProtocolPickerOutcome) -> PickerOutcome {
    match outcome {
        ProtocolPickerOutcome::Selected(id) => PickerOutcome::Selected(id),
        ProtocolPickerOutcome::Canceled => PickerOutcome::Canceled,
    }
}
