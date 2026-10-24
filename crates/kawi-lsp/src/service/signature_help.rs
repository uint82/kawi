use super::markup::markup_document;
use super::text_requests::{PendingTextRequest, TextResponse};
use super::*;
use lsp_types::{ParameterLabel, SignatureHelp};

pub(super) fn signature_help_event(
    pending: &PendingTextRequest,
    responses: &[TextResponse],
) -> Result<SignatureHelpEvent, String> {
    let mut response_error = None;
    for response in responses {
        if response.value.is_null() {
            continue;
        }
        match serde_json::from_value::<SignatureHelp>(response.value.clone()) {
            Ok(help) if !help.signatures.is_empty() => {
                let signatures: Vec<SignatureEntry> =
                    help.signatures.iter().map(signature_entry).collect();
                let active_signature = normalize_index(help.active_signature, signatures.len())
                    .or_else(|| (!signatures.is_empty()).then_some(0));
                let active_parameter = help.active_parameter.map(|index| index as usize);
                return Ok(SignatureHelpEvent {
                    request: pending.request,
                    server: response.server.clone(),
                    signatures,
                    active_signature,
                    active_parameter,
                });
            }
            Ok(_) => {}
            Err(error) => {
                response_error
                    .get_or_insert_with(|| format!("Invalid signature help response: {error}"));
            }
        }
    }
    Err(response_error.unwrap_or_else(|| "No signature help".to_string()))
}

fn signature_entry(signature: &lsp_types::SignatureInformation) -> SignatureEntry {
    let label = signature_label(&signature.label);
    let parameters: Vec<SignatureParameter> = signature
        .parameters
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|parameter| SignatureParameter {
            label: parameter_text(&label, &parameter.label),
            documentation: parameter.documentation.clone().map(markup_document),
            range: parameter_range(&label, &parameter.label),
        })
        .collect();
    let active_parameter = signature
        .active_parameter
        .and_then(|index| normalize_index(Some(index), parameters.len()));
    SignatureEntry {
        label,
        documentation: signature.documentation.clone().map(markup_document),
        parameters,
        active_parameter,
    }
}

fn signature_label(label: &str) -> String {
    label
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}

fn parameter_text(label: &str, parameter: &ParameterLabel) -> String {
    match parameter {
        ParameterLabel::Simple(value) => one_line_message(value),
        ParameterLabel::LabelOffsets(_) => parameter_range(label, parameter)
            .map(|(start, end)| label.chars().skip(start).take(end - start).collect())
            .unwrap_or_default(),
    }
}

fn parameter_range(label: &str, parameter: &ParameterLabel) -> Option<(usize, usize)> {
    match parameter {
        ParameterLabel::Simple(value) => {
            let start = label.find(value)?;
            let start = label[..start].chars().count();
            Some((start, start + value.chars().count()))
        }
        ParameterLabel::LabelOffsets([start, end]) => {
            let start = utf16_to_char_index(label, *start as usize)?;
            let end = utf16_to_char_index(label, *end as usize)?;
            (start < end).then_some((start, end))
        }
    }
}

fn utf16_to_char_index(label: &str, offset: usize) -> Option<usize> {
    let mut units = 0;
    for (index, character) in label.chars().enumerate() {
        if units == offset {
            return Some(index);
        }
        units = units.saturating_add(character.len_utf16());
        if units > offset {
            return None;
        }
    }
    (units == offset).then_some(label.chars().count())
}

fn normalize_index(index: Option<u32>, len: usize) -> Option<usize> {
    (len > 0)
        .then_some(index.unwrap_or(0) as usize)
        .filter(|index| *index < len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_offsets_use_utf16_code_units() {
        let label = "f(é: i32, 𝄞: i32)";
        assert_eq!(utf16_to_char_index(label, 10), Some(10));
        assert_eq!(utf16_to_char_index(label, 12), Some(11));
        assert_eq!(utf16_to_char_index(label, 11), None);
        assert_eq!(
            parameter_range(label, &ParameterLabel::LabelOffsets([10, 12])),
            Some((10, 11))
        );
    }

    #[test]
    fn simple_labels_locate_the_parameter_text() {
        let label = "f(é: i32)";
        assert_eq!(
            parameter_range(label, &ParameterLabel::Simple("é".to_string())),
            Some((2, 3))
        );
        assert_eq!(
            parameter_text(label, &ParameterLabel::LabelOffsets([2, 3])),
            "é"
        );
    }
}
