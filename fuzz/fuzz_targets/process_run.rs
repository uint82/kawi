#![no_main]

use kawi_plugin::fuzz::{mock_process_path, process_authorize};
use kawi_plugin::{CAPABILITY_DENIED, PLUGIN_UNAVAILABLE};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let mut lines = text.lines();
    let has_capability = lines
        .next()
        .is_some_and(|line| matches!(line.trim(), "1" | "true"));
    let granted = list(lines.next().unwrap_or_default());
    let command = lines.next().unwrap_or_default().trim().to_string();
    let args: Vec<String> = lines
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .map(str::to_string)
        .collect();

    let decision = process_authorize("fuzz", &command, &args, &granted, has_capability);
    assert_eq!(decision.args, args);

    let resolved = !command.is_empty();
    let expected = has_capability && resolved && granted.contains(&command);
    assert_eq!(decision.allowed, expected);

    if expected {
        let path = mock_process_path(&command).expect("path");
        assert_eq!(decision.path.as_deref(), Some(path.as_str()));
        assert_eq!(decision.argv.first(), Some(&path));
        assert_eq!(&decision.argv[1..], args.as_slice());
        assert!(decision.error.is_none());
        return;
    }

    assert!(decision.path.is_none());
    assert!(decision.argv.is_empty());
    let error = decision.error.expect("error");
    if !has_capability || resolved {
        assert_eq!(error.code, CAPABILITY_DENIED);
    } else {
        assert_eq!(error.code, PLUGIN_UNAVAILABLE);
    }
});

fn list(line: &str) -> Vec<String> {
    line.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}
