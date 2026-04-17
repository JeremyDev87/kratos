use std::path::Path;

use crate::error::{KratosError, KratosResult};
use crate::model::{EntrypointKind, ProjectConfig};

pub fn detect_entrypoint_kind(
    _file_path: &Path,
    _config: &ProjectConfig,
) -> KratosResult<Option<EntrypointKind>> {
    Err(KratosError::not_implemented(
        "entrypoints::detect_entrypoint_kind",
    ))
}
