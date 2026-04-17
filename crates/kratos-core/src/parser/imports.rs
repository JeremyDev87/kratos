use crate::error::{KratosError, KratosResult};
use crate::model::{ImportKind, ImportRecord, ImportSpecifier};

pub fn collect_imports(_source: &str) -> KratosResult<Vec<ImportRecord>> {
    Err(KratosError::not_implemented("parser::imports::collect_imports"))
}

pub fn make_unknown_import(source: impl Into<String>) -> ImportRecord {
    ImportRecord {
        source: source.into(),
        kind: ImportKind::Unknown,
        specifiers: vec![ImportSpecifier::unknown()],
    }
}
