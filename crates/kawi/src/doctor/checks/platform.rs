use super::super::report::{Check, Level};
use kawi_process::{sandbox_status, SandboxStatus};

#[cfg(target_os = "macos")]
const SANDBOX_HINT: &str =
    "Process plugins run without an OS sandbox on macOS; install only trusted development plugins.";

#[cfg(not(target_os = "macos"))]
const SANDBOX_HINT: &str =
    "Process plugins run without a landlock sandbox on this system; install only trusted development plugins.";

pub(crate) fn check() -> Check {
    let status = sandbox_status();
    let level = sandbox_level(status);
    let details = vec![platform_detail(), sandbox_detail(status)];
    let check = Check::new("platform", "Platform", level, details);
    if level == Level::Info {
        check.with_hint(SANDBOX_HINT)
    } else {
        check
    }
}

fn platform_detail() -> String {
    format!("os: {} {}", std::env::consts::OS, std::env::consts::ARCH)
}

fn sandbox_detail(status: SandboxStatus) -> String {
    #[cfg(target_os = "linux")]
    {
        format!("process sandbox: landlock {}", status_label(status))
    }
    #[cfg(not(target_os = "linux"))]
    {
        format!(
            "process sandbox: {} (landlock requires Linux)",
            status_label(status)
        )
    }
}

fn sandbox_level(status: SandboxStatus) -> Level {
    if status == SandboxStatus::Unavailable {
        Level::Info
    } else {
        Level::Ok
    }
}

fn status_label(status: SandboxStatus) -> &'static str {
    match status {
        SandboxStatus::Enforced => "enforced",
        SandboxStatus::Partial => "partial",
        SandboxStatus::Unavailable => "unavailable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_current_platform() {
        let check = check();
        assert_eq!(check.id, "platform");
        assert!(check.details[0].starts_with("os: "));
        assert!(check.details[1].contains("process sandbox:"));
    }

    #[test]
    fn unavailable_sandboxes_are_informational() {
        assert_eq!(sandbox_level(SandboxStatus::Unavailable), Level::Info);
        assert_eq!(sandbox_level(SandboxStatus::Partial), Level::Ok);
        assert_eq!(sandbox_level(SandboxStatus::Enforced), Level::Ok);
    }

    #[test]
    fn status_labels_describe_each_level() {
        assert_eq!(status_label(SandboxStatus::Enforced), "enforced");
        assert_eq!(status_label(SandboxStatus::Partial), "partial");
        assert_eq!(status_label(SandboxStatus::Unavailable), "unavailable");
    }
}
