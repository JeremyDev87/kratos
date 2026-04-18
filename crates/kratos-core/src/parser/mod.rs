use std::path::Path;

pub mod adapter;
pub mod exports;
pub mod imports;
pub mod unused_imports;

use crate::error::KratosResult;
use crate::model::{ExportRecord, ImportRecord, UnusedImportRecord};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParsedModule {
    pub imports: Vec<ImportRecord>,
    pub exports: Vec<ExportRecord>,
    pub unused_imports: Vec<UnusedImportRecord>,
}

pub fn parse_module_source(path: &Path, source: &str) -> KratosResult<ParsedModule> {
    adapter::parse_module_source(path, source)
}
