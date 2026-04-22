use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::suppressions::SuppressionRule;

pub const REPORT_V2: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectConfig {
    pub root: PathBuf,
    pub config_path: Option<PathBuf>,
    pub base_url: Option<PathBuf>,
    pub roots: Vec<PathBuf>,
    pub ignored_directories: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub explicit_entries: Vec<PathBuf>,
    pub package_entries: Vec<PathBuf>,
    pub path_aliases: Vec<PathAlias>,
    pub external_packages: BTreeSet<String>,
    pub suppressions: Vec<SuppressionRule>,
}

impl ProjectConfig {
    pub fn new(root: PathBuf) -> Self {
        Self {
            roots: vec![root.clone()],
            root,
            config_path: None,
            base_url: None,
            ignored_directories: Vec::new(),
            ignore_patterns: Vec::new(),
            explicit_entries: Vec::new(),
            package_entries: Vec::new(),
            path_aliases: Vec::new(),
            external_packages: BTreeSet::new(),
            suppressions: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathAlias {
    pub alias: String,
    pub target: PathBuf,
    pub target_pattern: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModuleRecord {
    pub file_path: PathBuf,
    pub relative_path: String,
    pub entrypoint_kind: Option<EntrypointKind>,
    pub imports: Vec<ImportRecord>,
    pub exports: Vec<ExportRecord>,
    pub unused_imports: Vec<UnusedImportRecord>,
    pub resolved_imports: Vec<ResolvedImportRecord>,
    pub importers: Vec<ImportUsageRecord>,
    pub imported_by: Vec<PathBuf>,
    pub imported_by_count: usize,
    pub import_count: usize,
    pub export_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntrypointKind {
    UserEntry,
    PackageEntry,
    NextAppRoute,
    NextPagesRoute,
    AppEntry,
    ToolingEntry,
    FrameworkEntry,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportRecord {
    pub source: String,
    pub kind: ImportKind,
    pub specifiers: Vec<ImportSpecifier>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportKind {
    Static,
    SideEffect,
    Reexport,
    ReexportAll,
    ReexportNamespace,
    Require,
    Dynamic,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportSpecifier {
    pub kind: ImportSpecifierKind,
    pub imported: Option<String>,
    pub local: Option<String>,
}

impl ImportSpecifier {
    pub fn unknown() -> Self {
        Self {
            kind: ImportSpecifierKind::Unknown,
            imported: None,
            local: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportSpecifierKind {
    Default,
    Named,
    Namespace,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedImportRecord {
    pub kind: ImportKind,
    pub source: String,
    pub target: PathBuf,
    pub specifiers: Vec<ImportSpecifier>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportUsageRecord {
    pub file_path: PathBuf,
    pub kind: ImportKind,
    pub specifiers: Vec<ImportSpecifier>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportRecord {
    pub name: String,
    pub kind: ExportKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportKind {
    Default,
    Named,
    Reexport,
    ReexportAll,
    ReexportNamespace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnusedImportRecord {
    pub source: String,
    pub local: String,
    pub imported: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportResolution {
    pub kind: ImportResolutionKind,
    pub source: String,
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportResolutionKind {
    Source,
    Asset,
    External,
    MissingInternal,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FindingSet {
    pub broken_imports: Vec<BrokenImportFinding>,
    pub orphan_files: Vec<OrphanFileFinding>,
    pub dead_exports: Vec<DeadExportFinding>,
    pub unused_imports: Vec<UnusedImportFinding>,
    pub route_entrypoints: Vec<RouteEntrypointFinding>,
    pub deletion_candidates: Vec<DeletionCandidateFinding>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SummaryCounts {
    pub files_scanned: usize,
    pub entrypoints: usize,
    pub broken_imports: usize,
    pub orphan_files: usize,
    pub dead_exports: usize,
    pub unused_imports: usize,
    pub route_entrypoints: usize,
    pub deletion_candidates: usize,
    pub suppressed_findings: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportV2 {
    pub version: u32,
    pub generated_at: Option<String>,
    pub root: PathBuf,
    pub config_path: Option<PathBuf>,
    pub summary: SummaryCounts,
    pub findings: FindingSet,
    pub modules: Vec<ModuleRecord>,
}

impl ReportV2 {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            ..Self::default()
        }
    }
}

impl Default for ReportV2 {
    fn default() -> Self {
        Self {
            version: REPORT_V2,
            generated_at: None,
            root: PathBuf::new(),
            config_path: None,
            summary: SummaryCounts::default(),
            findings: FindingSet::default(),
            modules: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokenImportFinding {
    pub file: PathBuf,
    pub source: String,
    pub kind: ImportKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrphanFileFinding {
    pub file: PathBuf,
    pub kind: OrphanKind,
    pub reason: String,
    pub confidence: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrphanKind {
    Module,
    Component,
    RouteModule,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeadExportFinding {
    pub file: PathBuf,
    pub export_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnusedImportFinding {
    pub file: PathBuf,
    pub source: String,
    pub local: String,
    pub imported: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteEntrypointFinding {
    pub file: PathBuf,
    pub kind: EntrypointKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeletionCandidateFinding {
    pub file: PathBuf,
    pub reason: String,
    pub confidence: f32,
    pub safe: bool,
}
