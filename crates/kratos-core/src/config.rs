use std::path::{Path, PathBuf};

use crate::error::{KratosError, KratosResult};
use crate::model::{PathAlias, ProjectConfig};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RawConfigDocument {
    pub package_json: Option<String>,
    pub tsconfig_json: Option<String>,
    pub kratos_json: Option<String>,
}

pub fn load_project_config(root: impl Into<PathBuf>) -> KratosResult<ProjectConfig> {
    let _ = root.into();
    Err(KratosError::not_implemented("config::load_project_config"))
}

pub fn resolve_config_path(root: &Path, file_name: &str) -> PathBuf {
    root.join(file_name)
}

pub fn apply_path_aliases(
    _config: &mut ProjectConfig,
    _aliases: Vec<PathAlias>,
) -> KratosResult<()> {
    Err(KratosError::not_implemented("config::apply_path_aliases"))
}
