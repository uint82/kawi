use anyhow::{bail, Result};
use kawi_plugin::api_metadata::{
    api_metadata, ApiEntryMetadata, ApiErrorMetadata, ApiMetadata, ApiSignatureMetadata,
};

enum Output {
    Text,
    Json,
}

fn parse_args(args: &[String]) -> Result<Output> {
    match args {
        [] => Ok(Output::Text),
        [flag] if flag == "--json" => Ok(Output::Json),
        [flag] => bail!("unknown option: {flag}\nusage: kawi plugin api [--json]"),
        _ => bail!("usage: kawi plugin api [--json]"),
    }
}

fn optional_number(value: Option<u64>) -> String {
    value.map_or_else(|| "-".to_string(), |number| number.to_string())
}

fn render_signature(signature: Option<&ApiSignatureMetadata>) -> String {
    let Some(signature) = signature else {
        return "-".to_string();
    };
    let parameters: Vec<String> = signature
        .parameters
        .iter()
        .map(|parameter| {
            let marker = if parameter.required { "" } else { "?" };
            format!("{}{}: {}", parameter.name, marker, parameter.ty)
        })
        .collect();
    format!("({}) -> {}", parameters.join(", "), signature.returns)
}

fn render_flags(entry: &ApiEntryMetadata) -> String {
    let mut flags = String::new();
    if entry.interactive {
        flags.push('i');
    }
    if entry.mutates {
        flags.push('m');
    }
    if flags.is_empty() {
        "-".to_string()
    } else {
        flags
    }
}

fn render_entry(entry: &ApiEntryMetadata) -> String {
    format!(
        "{:<26} {:<6} {:<12} {:<20} {:<5} {:<14} {:<20} {}",
        entry.name,
        entry.since,
        entry.stability,
        entry.capability.unwrap_or("-"),
        render_flags(entry),
        optional_number(entry.deprecated_since),
        entry.replacement.unwrap_or("-"),
        render_signature(entry.signature.as_ref())
    )
}

fn render_section(title: &str, entries: &[ApiEntryMetadata]) -> String {
    let mut lines = vec![
        format!("{title} ({}):", entries.len()),
        format!(
            "{:<26} {:<6} {:<12} {:<20} {:<5} {:<14} {:<20} {}",
            "NAME",
            "SINCE",
            "STABILITY",
            "CAPABILITY",
            "FLAGS",
            "DEPRECATED",
            "REPLACEMENT",
            "SIGNATURE"
        ),
        "flags: i=interactive, m=mutates".to_string(),
    ];
    lines.extend(entries.iter().map(render_entry));
    lines.join("\n")
}

fn render_errors(entries: &[ApiErrorMetadata]) -> String {
    let mut lines = vec![
        format!("Errors ({}):", entries.len()),
        format!("{:<8} {:<22} {}", "CODE", "NAME", "MEANING"),
    ];
    lines.extend(
        entries
            .iter()
            .map(|entry| format!("{:<8} {:<22} {}", entry.code, entry.name, entry.description)),
    );
    lines.join("\n")
}

fn render_text(report: &ApiMetadata) -> String {
    format!(
        "protocol: {} (minor {})\n\n{}\n\n{}\n\n{}",
        report.version.protocol,
        report.version.protocol_minor,
        render_section("Host methods", &report.methods),
        render_section("Events", &report.events),
        render_errors(&report.error_types)
    )
}

pub(super) fn run(args: &[String]) -> Result<()> {
    let output = parse_args(args)?;
    let report = api_metadata();
    let text = match output {
        Output::Text => render_text(&report),
        Output::Json => serde_json::to_string_pretty(&report)?,
    };
    println!("{text}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_plugin::api_metadata::ApiParameterMetadata;

    fn string(value: &str) -> String {
        value.to_string()
    }

    #[test]
    fn parse_args_accepts_the_default_and_json_flag() {
        assert!(matches!(parse_args(&[]), Ok(Output::Text)));
        assert!(matches!(parse_args(&[string("--json")]), Ok(Output::Json)));
    }

    #[test]
    fn parse_args_rejects_unknown_and_extra_arguments() {
        assert!(parse_args(&[string("--yaml")]).is_err());
        assert!(parse_args(&[string("--json"), string("extra")]).is_err());
    }

    #[test]
    fn optional_number_uses_a_dash_for_missing_values() {
        assert_eq!(optional_number(None), "-");
        assert_eq!(optional_number(Some(19)), "19");
    }

    #[test]
    fn render_entry_aligns_all_columns() {
        let entry = ApiEntryMetadata {
            name: "old.method",
            since: 4,
            stability: "deprecated",
            deprecated_since: Some(6),
            replacement: Some("new.method"),
            capability: Some("editor.read"),
            interactive: false,
            mutates: true,
            signature: Some(ApiSignatureMetadata {
                parameters: &[ApiParameterMetadata {
                    name: "flag",
                    ty: "boolean",
                    required: false,
                }],
                returns: "null",
            }),
        };
        assert_eq!(
            render_entry(&entry),
            "old.method                 4      deprecated   editor.read          m     6              new.method           (flag?: boolean) -> null"
        );
    }

    #[test]
    fn render_signature_marks_optional_parameters_and_returns() {
        let signature = ApiSignatureMetadata {
            parameters: &[
                ApiParameterMetadata {
                    name: "buffer",
                    ty: "integer",
                    required: true,
                },
                ApiParameterMetadata {
                    name: "include_text",
                    ty: "boolean",
                    required: false,
                },
            ],
            returns: "BufferState",
        };
        assert_eq!(
            render_signature(Some(&signature)),
            "(buffer: integer, include_text?: boolean) -> BufferState"
        );
        assert_eq!(render_signature(None), "-");
    }

    #[test]
    fn render_section_includes_a_header_for_empty_input() {
        let text = render_section("Events", &[]);
        assert!(text.contains("Events (0):"));
        assert!(text.contains("NAME"));
        assert!(!text.contains("old.method"));
    }

    #[test]
    fn render_errors_lists_the_code_name_and_meaning() {
        let entries = [
            ApiErrorMetadata {
                name: "CAPABILITY_DENIED",
                code: -32012,
                description: "Capability denied",
            },
            ApiErrorMetadata {
                name: "STALE_REVISION",
                code: -32010,
                description: "Stale buffer revision",
            },
        ];
        let text = render_errors(&entries);
        assert!(text.starts_with("Errors (2):"));
        assert!(text.contains("CODE"));
        assert!(text.contains("-32012"));
        assert!(text.contains("CAPABILITY_DENIED"));
        assert!(text.contains("Capability denied"));
    }

    #[test]
    fn render_text_shows_the_version_and_every_section() {
        let text = render_text(&api_metadata());
        assert!(text.contains("protocol: 1"));
        assert!(text.contains("Host methods ("));
        assert!(text.contains("Events ("));
        assert!(text.contains("Errors ("));
        assert!(text.contains("editor.get_state"));
        assert!(text.contains("(include_text?: boolean) -> GetStateResult"));
    }

    #[test]
    fn json_output_parses_with_the_expected_shape() {
        let report = api_metadata();
        let json = serde_json::to_string_pretty(&report).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["version"]["protocol"], 1);
        assert!(value["methods"].as_array().unwrap().len() > 50);
        assert!(value["events"].as_array().unwrap().len() > 10);
        assert!(!value["error_types"].as_array().unwrap().is_empty());
    }
}
