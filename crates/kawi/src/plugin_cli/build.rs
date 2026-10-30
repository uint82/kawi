use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};
use toml_edit::{value, DocumentMut};

const MANIFEST_FILE: &str = "plugin.toml";
const TARGET: &str = "wasm32-wasip2";
const PROFILE: &str = "release";

pub(super) fn run(args: &[String]) -> Result<()> {
    let directory = parse_directory(args)?;
    let manifest_path = directory.join(MANIFEST_FILE);
    let text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("cannot read {}", manifest_path.display()))?;
    let document = text
        .parse::<DocumentMut>()
        .with_context(|| format!("cannot parse {}", manifest_path.display()))?;
    require_wasm_runtime(&document)?;
    let component = component_path(&document)?;
    let artifact = build_artifact(&directory)?;
    copy_component(&artifact, &directory, &component)?;
    let hash = update_manifest(&manifest_path, &document, &directory.join(&component))?;
    println!("built {} ({hash})", component.display());
    Ok(())
}

fn parse_directory(args: &[String]) -> Result<PathBuf> {
    match args {
        [] => Ok(PathBuf::from(".")),
        [directory] if !directory.starts_with('-') => Ok(PathBuf::from(directory)),
        [argument] => bail!("unexpected argument: {argument}"),
        _ => bail!("usage: kawi plugin build [directory]"),
    }
}

fn require_wasm_runtime(document: &DocumentMut) -> Result<()> {
    let runtime = document
        .get("runtime")
        .and_then(|item| item.as_str())
        .unwrap_or("process");
    if runtime != "wasm" {
        bail!("plugin build requires runtime = \"wasm\"");
    }
    Ok(())
}

fn component_path(document: &DocumentMut) -> Result<PathBuf> {
    let component = document
        .get("component")
        .and_then(|item| item.as_str())
        .context("wasm plugin manifest is missing component")?;
    kawi_plugin::validate_component_path(component).map_err(anyhow::Error::msg)?;
    Ok(PathBuf::from(component))
}

fn build_artifact(directory: &Path) -> Result<PathBuf> {
    let metadata = cargo_metadata(directory)?;
    let target = library_target(&metadata, directory)?;
    let status = Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(directory.join("Cargo.toml"))
        .arg("--target")
        .arg(TARGET)
        .arg("--release")
        .status()
        .context("cannot run cargo build")?;
    if !status.success() {
        bail!("cargo build failed");
    }
    let target_directory = metadata
        .get("target_directory")
        .and_then(Value::as_str)
        .context("cargo metadata is missing target_directory")?;
    Ok(PathBuf::from(target_directory)
        .join(TARGET)
        .join(PROFILE)
        .join(format!("{target}.wasm")))
}

fn cargo_metadata(directory: &Path) -> Result<Value> {
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .arg("--manifest-path")
        .arg(directory.join("Cargo.toml"))
        .output()
        .context("cannot run cargo metadata")?;
    if !output.status.success() {
        bail!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    serde_json::from_slice(&output.stdout).context("cannot parse cargo metadata")
}

fn library_target(metadata: &Value, directory: &Path) -> Result<String> {
    let manifest_path = directory
        .join("Cargo.toml")
        .canonicalize()
        .context("cannot resolve Cargo.toml")?;
    let target = metadata
        .get("packages")
        .and_then(Value::as_array)
        .and_then(|packages| {
            packages.iter().find(|package| {
                package
                    .get("manifest_path")
                    .and_then(Value::as_str)
                    .is_some_and(|path| Path::new(path) == manifest_path)
            })
        })
        .and_then(|package| package.get("targets"))
        .and_then(Value::as_array)
        .and_then(|targets| {
            targets.iter().find(|target| {
                target
                    .get("crate_types")
                    .and_then(Value::as_array)
                    .is_some_and(|types| types.iter().any(|kind| kind == "cdylib"))
            })
        })
        .context("cargo package has no cdylib library target")?;
    target
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_string)
        .context("library target is missing a name")
}

fn copy_component(artifact: &Path, directory: &Path, component: &Path) -> Result<()> {
    if !artifact.is_file() {
        bail!("cargo did not produce {}", artifact.display());
    }
    let destination = directory.join(component);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    fs::copy(artifact, &destination)
        .with_context(|| format!("cannot copy component to {}", destination.display()))?;
    Ok(())
}

fn update_manifest(
    manifest_path: &Path,
    document: &DocumentMut,
    component_path: &Path,
) -> Result<String> {
    let bytes = fs::read(component_path)
        .with_context(|| format!("cannot read {}", component_path.display()))?;
    let hash = format!("sha256:{:x}", Sha256::digest(&bytes));
    let mut document = document.clone();
    document["sha256"] = value(hash.clone());
    fs::write(manifest_path, document.to_string())
        .with_context(|| format!("cannot write {}", manifest_path.display()))?;
    Ok(hash)
}
