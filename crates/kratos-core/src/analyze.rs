use std::path::Path;

use crate::error::{KratosError, KratosResult};
use crate::model::{ProjectConfig, ReportV2};

pub fn analyze_project(_root: &Path) -> KratosResult<ReportV2> {
    Err(KratosError::not_implemented("analyze::analyze_project"))
}

pub fn analyze_with_config(_config: &ProjectConfig) -> KratosResult<ReportV2> {
    Err(KratosError::not_implemented(
        "analyze::analyze_with_config",
    ))
}
