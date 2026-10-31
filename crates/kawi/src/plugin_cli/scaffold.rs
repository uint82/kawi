use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

const SDK_VERSION: &str = "0.1.0";
const DEFAULT_SHA256: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";
const BUILD_SCRIPT: &str = "\
#!/bin/sh
set -eu

dir=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)
kawi plugin build \"$dir\"
";

pub(super) struct Options {
    pub(super) name: String,
    pub(super) sdk_path: Option<PathBuf>,
}

pub(super) fn run(args: &[String]) -> Result<()> {
    let options = parse_args(args)?;
    let current = std::env::current_dir()?;
    let options = Options {
        name: options.name,
        sdk_path: options.sdk_path.map(|path| current.join(path)),
    };
    let directory = current.join(&options.name);
    create_project(&directory, &options)?;
    println!("created {}", directory.display());
    println!("build with: kawi plugin build {}", directory.display());
    Ok(())
}

pub(super) fn create_project(directory: &Path, options: &Options) -> Result<()> {
    kawi_plugin::validate_plugin_name(&options.name).map_err(anyhow::Error::msg)?;
    fs::create_dir(directory).with_context(|| format!("cannot create {}", directory.display()))?;
    if let Err(error) = write_project(directory, options) {
        let _ = fs::remove_dir_all(directory);
        return Err(error);
    }
    Ok(())
}

fn parse_args(args: &[String]) -> Result<Options> {
    let mut name = None;
    let mut sdk_path = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sdk-path" => {
                let path = args
                    .next()
                    .context("--sdk-path requires a directory argument")?;
                sdk_path = Some(PathBuf::from(path));
            }
            value if value.starts_with("--") => bail!("unknown option: {value}"),
            value if name.is_none() => name = Some(value.to_string()),
            value => bail!("unexpected argument: {value}"),
        }
    }
    Ok(Options {
        name: name.context("missing plugin name")?,
        sdk_path,
    })
}

fn write_project(directory: &Path, options: &Options) -> Result<()> {
    fs::create_dir(directory.join("src"))?;
    write_file(&directory.join("Cargo.toml"), &cargo_manifest(options))?;
    write_file(&directory.join("src/lib.rs"), &source(&options.name))?;
    write_file(
        &directory.join("plugin.toml"),
        &plugin_manifest(&options.name),
    )?;
    write_file(&directory.join("build.sh"), BUILD_SCRIPT)?;
    write_file(&directory.join(".gitignore"), "/target\n")?;
    make_executable(&directory.join("build.sh"))?;
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    fs::write(path, contents).with_context(|| format!("cannot write {}", path.display()))
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("cannot make {} executable", path.display()))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

fn cargo_manifest(options: &Options) -> String {
    format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[dependencies]\n{}\n\n[workspace]\n",
        options.name,
        sdk_dependency(options.sdk_path.as_deref())
    )
}

fn sdk_dependency(path: Option<&Path>) -> String {
    match path {
        Some(path) => format!(
            "kawi-plugin-sdk = {{ version = \"{SDK_VERSION}\", path = {} }}",
            toml_string(&path.to_string_lossy())
        ),
        None => format!("kawi-plugin-sdk = \"{SDK_VERSION}\""),
    }
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn plugin_manifest(name: &str) -> String {
    format!(
        "name = \"{name}\"\nversion = \"0.1.0\"\nprotocol = 1\nruntime = \"wasm\"\ncomponent = \"{name}.wasm\"\nsha256 = \"{DEFAULT_SHA256}\"\ntimeout_ms = 5000\n\n[[activation_events]]\nevent = \"on_command\"\n\n[[commands]]\nid = \"hello\"\ntitle = \"Say hello\"\ndescription = \"Print a greeting from the plugin\"\n"
    )
}

fn source(name: &str) -> String {
    format!(
        "use kawi_plugin_sdk::exports::kawi::plugin::lifecycle::Guest;\nuse kawi_plugin_sdk::types::{{Argument, Error, Event}};\nuse kawi_plugin_sdk::INVALID_PARAMS;\n\nconst COMMAND_ID: &str = \"hello\";\nconst MESSAGE: &str = \"hello from {name}\";\n\nstruct Plugin;\n\nimpl Guest for Plugin {{\n    fn init(_host_protocol: u32) -> Result<(), Error> {{\n        Ok(())\n    }}\n\n    fn invoke_command(id: String, _args: Vec<Argument>) -> Result<Argument, Error> {{\n        if id != COMMAND_ID {{\n            return Err(Error {{\n                code: INVALID_PARAMS,\n                message: \"unknown command\".to_string(),\n                capability: None,\n            }});\n        }}\n        Ok(Argument::Text(MESSAGE.to_string()))\n    }}\n\n    fn on_event(_event: Event) {{}}\n}}\n\nkawi_plugin_sdk::export!(Plugin);\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_name_and_sdk_path() {
        let options = parse_args(&[
            "demo".to_string(),
            "--sdk-path".to_string(),
            "../sdk".to_string(),
        ])
        .unwrap();
        assert_eq!(options.name, "demo");
        assert_eq!(options.sdk_path, Some(PathBuf::from("../sdk")));
    }

    #[test]
    fn sdk_path_keeps_the_version_requirement() {
        let dependency = sdk_dependency(Some(Path::new("/tmp/sdk")));
        assert!(dependency.contains("version = \"0.1.0\""));
        assert!(dependency.contains("path = \"/tmp/sdk\""));
    }

    #[test]
    fn default_dependency_uses_the_published_sdk() {
        assert_eq!(sdk_dependency(None), "kawi-plugin-sdk = \"0.1.0\"");
    }

    #[test]
    fn manifest_uses_the_plugin_name() {
        let manifest = plugin_manifest("demo");
        assert!(manifest.contains("component = \"demo.wasm\""));
        assert!(manifest.contains("name = \"demo\""));
    }

    #[test]
    fn source_exports_the_guest() {
        let source = source("demo");
        assert!(source.contains("impl Guest for Plugin"));
        assert!(source.contains("hello from demo"));
        assert!(source.contains("kawi_plugin_sdk::export!(Plugin);"));
    }
}
