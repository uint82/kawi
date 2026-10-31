mod build;
mod inspect;
mod scaffold;

#[cfg(all(test, feature = "wasm"))]
mod tests;

use anyhow::{bail, Result};

const USAGE: &str = "\
Usage:
  kawi plugin new <name> [--sdk-path <path>]
  kawi plugin build [directory]
  kawi plugin inspect [directory]

The build and inspect commands require a wasm plugin directory.
The default SDK is fetched from crates.io. Use --sdk-path for an unreleased checkout.";

pub(crate) fn run(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("new") => scaffold::run(&args[1..]),
        Some("build") => build::run(&args[1..]),
        Some("inspect") => inspect::run(&args[1..]),
        Some("-h" | "--help") => {
            println!("{USAGE}");
            Ok(())
        }
        Some(command) => bail!("unknown plugin command: {command}\n{USAGE}"),
        None => {
            println!("{USAGE}");
            Ok(())
        }
    }
}
