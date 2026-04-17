use crate::error::{KratosError, KratosResult};
use crate::model::UnusedImportRecord;

pub fn detect_unused_imports(_source: &str) -> KratosResult<Vec<UnusedImportRecord>> {
    Err(KratosError::not_implemented(
        "parser::unused_imports::detect_unused_imports",
    ))
}
