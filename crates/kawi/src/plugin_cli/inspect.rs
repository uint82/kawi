use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use kawi_plugin::{load_catalog, PluginRuntime};

pub(super) fn run(args: &[String]) -> Result<()> {
    let directory = parse_directory(args)?;
    let directory = directory
        .canonicalize()
        .with_context(|| format!("cannot resolve {}", directory.display()))?;
    let parent = directory
        .parent()
        .with_context(|| format!("{} has no parent directory", directory.display()))?;
    let catalog = load_catalog(parent);
    let plugin = catalog
        .plugins
        .iter()
        .find(|plugin| plugin.dir.canonicalize().ok().as_deref() == Some(directory.as_path()));
    let Some(plugin) = plugin else {
        for warning in &catalog.warnings {
            eprintln!("{warning}");
        }
        bail!("no plugin manifest found in {}", directory.display());
    };
    print_plugin(plugin)?;
    if plugin.manifest.runtime == PluginRuntime::Wasm && plugin.disabled_reason.is_none() {
        inspect_component(plugin)?;
    }
    Ok(())
}

fn parse_directory(args: &[String]) -> Result<PathBuf> {
    match args {
        [] => Ok(PathBuf::from(".")),
        [directory] if !directory.starts_with('-') => Ok(PathBuf::from(directory)),
        [argument] => bail!("unexpected argument: {argument}"),
        _ => bail!("usage: kawi plugin inspect [directory]"),
    }
}

fn print_plugin(plugin: &kawi_plugin::LoadedPlugin) -> Result<()> {
    println!("name: {}", plugin.manifest.name);
    println!("version: {}", plugin.manifest.version);
    println!("runtime: {:?}", plugin.manifest.runtime);
    if let Some(component) = plugin.manifest.component.as_deref() {
        println!("component: {component}");
    }
    if let Some(hash) = plugin.component_hash() {
        println!("sha256: {hash}");
        println!(
            "request hash: {}",
            plugin.manifest.capability_requests.request_hash()
        );
    }
    if let Some(hash) = plugin.declared_hash() {
        println!("declared sha256: {hash}");
    }
    if let Some(reason) = plugin.disabled_reason.as_deref() {
        println!("disabled: {reason}");
    }
    for (capability, scope) in plugin.manifest.capability_requests.iter() {
        println!(
            "requests: {} {}",
            capability.name(),
            serde_json::Value::Object(scope.to_json_object())
        );
    }
    Ok(())
}

#[cfg(feature = "wasm")]
fn inspect_component(plugin: &kawi_plugin::LoadedPlugin) -> Result<()> {
    let path = plugin
        .component_path()
        .context("wasm plugin has no component path")?;
    let requested = plugin
        .manifest
        .capability_requests
        .iter()
        .map(|(capability, _)| capability)
        .collect::<Vec<_>>();
    let inspection = kawi_plugin::WasmRuntime::new()?
        .inspect(&path, plugin.component_hash(), &requested)
        .map_err(anyhow::Error::msg)?;
    println!("component hash: {}", inspection.hash);
    println!("imports:");
    for import in inspection.imports {
        println!("  {import}");
    }
    Ok(())
}

#[cfg(not(feature = "wasm"))]
fn inspect_component(_plugin: &kawi_plugin::LoadedPlugin) -> Result<()> {
    bail!("plugin inspect requires the wasm feature")
}
