use std::path::{Path, PathBuf};

use crate::error::{KratosError, KratosResult};
use crate::model::ProjectConfig;

pub fn normalize_root(root: &Path) -> PathBuf {
    root.to_path_buf()
}

pub fn collect_source_files(_config: &ProjectConfig) -> KratosResult<Vec<PathBuf>> {
    Err(KratosError::not_implemented("discover::collect_source_files"))
}
