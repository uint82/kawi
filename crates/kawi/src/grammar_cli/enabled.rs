use std::path::Path;

use anyhow::{anyhow, bail, Result};
use kawi_grammar::{GrammarLoader, GrammarStore, Pack, PackRoot, PackState, PackStatus};
use kawi_syntax::LanguageId;

use super::USAGE;
use crate::grammar::load_pack;
use crate::state::StatePaths;

pub(crate) fn run(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("install") => install(&args[1..]),
        Some("status") => status(&args[1..]),
        Some("-h" | "--help") => {
            println!("{USAGE}");
            Ok(())
        }
        Some(command) => bail!("unknown grammar command: {command}\n{USAGE}"),
        None => {
            println!("{USAGE}");
            Ok(())
        }
    }
}

fn install(args: &[String]) -> Result<()> {
    let [path] = args else {
        bail!("usage: kawi grammar install <path>");
    };
    let paths = state_paths()?;
    let source = Path::new(path);
    let pack = Pack::open(source).map_err(|error| anyhow!("{error}"))?;
    let name = pack.manifest().language.clone();
    if LanguageId::from_alias(&name).is_none() {
        bail!("unsupported grammar language: {name}");
    }
    let loader = GrammarLoader::new().map_err(|error| anyhow!("{error}"))?;
    load_pack(&loader, &pack).map_err(|error| anyhow!("{error}"))?;
    let installed = store(&paths)
        .install(source)
        .map_err(|error| anyhow!("{error}"))?;
    println!("installed {} {}", installed.id, installed.version);
    println!("{}", installed.path.display());
    Ok(())
}

fn status(args: &[String]) -> Result<()> {
    if !args.is_empty() {
        bail!("usage: kawi grammar status");
    }
    let paths = state_paths()?;
    let statuses = store(&paths).status();
    if statuses.is_empty() {
        println!("no grammar packs installed");
        return Ok(());
    }
    let loader = GrammarLoader::new().map_err(|error| anyhow!("{error}"))?;
    for status in statuses {
        println!(
            "{} {} [{}] {} {}",
            status.id,
            status.version,
            status.root.label(),
            describe(&status, &loader),
            status.path.display()
        );
    }
    Ok(())
}

fn describe(status: &PackStatus, loader: &GrammarLoader) -> String {
    match &status.state {
        PackState::Invalid(error) => format!("invalid: {error}"),
        PackState::Available => "unverified".to_string(),
        PackState::Valid => match Pack::open(&status.path) {
            Ok(pack) => {
                let language = &pack.manifest().language;
                if LanguageId::from_alias(language).is_none() {
                    return format!("unloadable: unsupported language {language}");
                }
                match load_pack(loader, &pack) {
                    Ok(_) => "valid".to_string(),
                    Err(error) => format!("unloadable: {error}"),
                }
            }
            Err(error) => format!("unloadable: {error}"),
        },
    }
}

fn state_paths() -> Result<StatePaths> {
    StatePaths::resolve().ok_or_else(|| anyhow!("cannot resolve the kawi data directory"))
}

fn store(paths: &StatePaths) -> GrammarStore {
    GrammarStore::new(vec![PackRoot::user(&paths.grammars)])
}
