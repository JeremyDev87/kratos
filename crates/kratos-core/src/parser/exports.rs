use crate::error::{KratosError, KratosResult};
use crate::model::{ExportKind, ExportRecord};

pub fn collect_exports(_source: &str) -> KratosResult<Vec<ExportRecord>> {
    Err(KratosError::not_implemented("parser::exports::collect_exports"))
}

pub fn make_default_export() -> ExportRecord {
    ExportRecord {
        name: "default".to_string(),
        kind: ExportKind::Default,
    }
}
