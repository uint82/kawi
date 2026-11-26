#[cfg(not(feature = "wasm-grammars"))]
mod disabled;
#[cfg(feature = "wasm-grammars")]
mod enabled;

use anyhow::Result;

const USAGE: &str = "\
Usage:
  kawi grammar install <path>
  kawi grammar status

Grammar packs are installed under $XDG_DATA_HOME/kawi/grammars.";

pub(crate) fn run(args: &[String]) -> Result<()> {
    #[cfg(feature = "wasm-grammars")]
    {
        enabled::run(args)
    }
    #[cfg(not(feature = "wasm-grammars"))]
    {
        disabled::run(args)
    }
}
