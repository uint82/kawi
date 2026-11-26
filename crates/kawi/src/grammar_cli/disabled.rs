use anyhow::{bail, Result};

use super::USAGE;

pub(crate) fn run(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("-h" | "--help") | None => {
            println!("{USAGE}");
            Ok(())
        }
        Some(_) => bail!(
            "grammar support is disabled; rebuild kawi with --features wasm-grammars\n{USAGE}"
        ),
    }
}
