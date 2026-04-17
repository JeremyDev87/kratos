use std::path::Path;

use crate::error::{KratosError, KratosResult};
use crate::model::{ImportResolution, ImportResolutionKind, ProjectConfig};

pub fn unresolved_import(source: impl Into<String>) -> ImportResolution {
    ImportResolution {
        kind: ImportResolutionKind::MissingInternal,
        source: source.into(),
        path: None,
    }
}

pub fn resolve_import_target(
    _source: &str,
    _from: &Path,
    _config: &ProjectConfig,
) -> KratosResult<ImportResolution> {
    Err(KratosError::not_implemented(
        "resolve::resolve_import_target",
    ))
}
