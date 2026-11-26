use std::error::Error;
use std::fmt;

use tree_sitter::{LANGUAGE_VERSION, MIN_COMPATIBLE_LANGUAGE_VERSION};

pub const MIN_ABI: usize = MIN_COMPATIBLE_LANGUAGE_VERSION;
pub const MAX_ABI: usize = LANGUAGE_VERSION;

#[derive(Debug, PartialEq, Eq)]
pub struct AbiError {
    pub found: usize,
    pub min: usize,
    pub max: usize,
}

impl fmt::Display for AbiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "grammar ABI {} is outside the supported range {}..={}",
            self.found, self.min, self.max
        )
    }
}

impl Error for AbiError {}

pub fn check_abi(abi: usize) -> Result<(), AbiError> {
    if (MIN_ABI..=MAX_ABI).contains(&abi) {
        Ok(())
    } else {
        Err(AbiError {
            found: abi,
            min: MIN_ABI,
            max: MAX_ABI,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_minimum_abi() {
        assert_eq!(check_abi(MIN_ABI), Ok(()));
    }

    #[test]
    fn accepts_the_maximum_abi() {
        assert_eq!(check_abi(MAX_ABI), Ok(()));
    }

    #[test]
    fn rejects_an_abi_below_the_minimum() {
        let abi = MIN_ABI.checked_sub(1).expect("ABI minimum is above zero");
        assert_eq!(
            check_abi(abi),
            Err(AbiError {
                found: abi,
                min: MIN_ABI,
                max: MAX_ABI,
            })
        );
    }

    #[test]
    fn rejects_an_abi_above_the_maximum() {
        let abi = MAX_ABI + 1;
        assert_eq!(
            check_abi(abi),
            Err(AbiError {
                found: abi,
                min: MIN_ABI,
                max: MAX_ABI,
            })
        );
    }
}
