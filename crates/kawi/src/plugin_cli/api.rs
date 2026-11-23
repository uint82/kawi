use anyhow::{bail, Result};
use kawi_plugin::api_metadata::{api_metadata, ApiEntryMetadata, ApiErrorMetadata, ApiMetadata};

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

fn render_entry(entry: &ApiEntryMetadata) -> String {
    format!(
        "{:<26} {:<6} {:<12} {:<14} {}",
        entry.name,
        entry.since,
        entry.stability,
        optional_number(entry.deprecated_since),
        entry.replacement.unwrap_or("-")
    )
}

fn render_section(title: &str, entries: &[ApiEntryMetadata]) -> String {
    let mut lines = vec![
        format!("{title} ({}):", entries.len()),
        format!(
            "{:<26} {:<6} {:<12} {:<14} {}",
            "NAME", "SINCE", "STABILITY", "DEPRECATED", "REPLACEMENT"
        ),
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
        "protocol: {} (minor {}, process minor {})\n\n{}\n\n{}\n\n{}",
        report.version.protocol,
        report.version.protocol_minor,
        report.version.process_protocol_minor,
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
    fn render_entry_aligns_all_five_columns() {
        let entry = ApiEntryMetadata {
            name: "old.method",
            since: 4,
            stability: "deprecated",
            deprecated_since: Some(6),
            replacement: Some("new.method"),
        };
        assert_eq!(
            render_entry(&entry),
            "old.method                 4      deprecated   6              new.method"
        );
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
