use crate::error::{KratosError, KratosResult};

use super::ParsedModule;

pub fn parse_module_source(_source: &str) -> KratosResult<ParsedModule> {
    Err(KratosError::not_implemented(
        "parser::adapter::parse_module_source",
    ))
}
