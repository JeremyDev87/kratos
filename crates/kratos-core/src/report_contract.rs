//! Frozen v1 consumer key names for Kratos report JSON.
//!
//! `REPORT_SCHEMA_VERSION` is the schema emitted by `scan --json` and persisted
//! report files for the v1 CLI contract. Future readers may accept compatible
//! newer schema versions, but v1 writers must not rename or remove these keys
//! without an explicit schema/version migration.

use crate::model::REPORT_CURRENT;

/// Current schema version emitted after the explicit clean-safety migration.
pub(crate) const REPORT_SCHEMA_VERSION: u32 = REPORT_CURRENT;

pub(crate) mod top_level {
    pub(crate) const SCHEMA_VERSION: &str = "schemaVersion";
    pub(crate) const GENERATED_AT: &str = "generatedAt";
    pub(crate) const PROJECT: &str = "project";
    pub(crate) const SUMMARY: &str = "summary";
    pub(crate) const FINDINGS: &str = "findings";
    pub(crate) const CLEAN_SAFETY: &str = "cleanSafety";
    pub(crate) const GRAPH: &str = "graph";
}

pub(crate) mod clean_safety {
    pub(crate) const FINGERPRINT_ALGORITHM: &str = "fingerprintAlgorithm";
    pub(crate) const CANDIDATES: &str = "candidates";
    pub(crate) const FILE: &str = "file";
    pub(crate) const FINGERPRINT: &str = "fingerprint";
    pub(crate) const IDENTITY: &str = "identity";
    pub(crate) const PARENT_IDENTITY: &str = "parentIdentity";
}

pub(crate) mod project {
    pub(crate) const ROOT: &str = "root";
    pub(crate) const CONFIG_PATH: &str = "configPath";
}

pub(crate) mod summary {
    pub(crate) const FILES_SCANNED: &str = "filesScanned";
    pub(crate) const ENTRYPOINTS: &str = "entrypoints";
    pub(crate) const BROKEN_IMPORTS: &str = "brokenImports";
    pub(crate) const ORPHAN_FILES: &str = "orphanFiles";
    pub(crate) const DEAD_EXPORTS: &str = "deadExports";
    pub(crate) const UNUSED_IMPORTS: &str = "unusedImports";
    pub(crate) const ROUTE_ENTRYPOINTS: &str = "routeEntrypoints";
    pub(crate) const DELETION_CANDIDATES: &str = "deletionCandidates";
    pub(crate) const SUPPRESSED_FINDINGS: &str = "suppressedFindings";
}

pub(crate) mod findings {
    pub(crate) const BROKEN_IMPORTS: &str = "brokenImports";
    pub(crate) const ORPHAN_FILES: &str = "orphanFiles";
    pub(crate) const DEAD_EXPORTS: &str = "deadExports";
    pub(crate) const UNUSED_IMPORTS: &str = "unusedImports";
    pub(crate) const ROUTE_ENTRYPOINTS: &str = "routeEntrypoints";
    pub(crate) const DELETION_CANDIDATES: &str = "deletionCandidates";
}

pub(crate) mod finding_field {
    pub(crate) const FILE: &str = "file";
    pub(crate) const SOURCE: &str = "source";
    pub(crate) const KIND: &str = "kind";
    pub(crate) const REASON: &str = "reason";
    pub(crate) const CONFIDENCE: &str = "confidence";
    pub(crate) const EXPORT_NAME: &str = "exportName";
    pub(crate) const EXPORT_KIND: &str = "exportKind";
    pub(crate) const IMPORTED_BY_COUNT: &str = "importedByCount";
    pub(crate) const USED_EXPORT_NAMES: &str = "usedExportNames";
    pub(crate) const HAS_NAMESPACE_OR_UNKNOWN_USAGE: &str = "hasNamespaceOrUnknownUsage";
    pub(crate) const LOCAL: &str = "local";
    pub(crate) const IMPORTED: &str = "imported";
    pub(crate) const SAFE: &str = "safe";
}

pub(crate) mod graph {
    pub(crate) const MODULES: &str = "modules";
}

pub(crate) mod module {
    pub(crate) const FILE: &str = "file";
    pub(crate) const RELATIVE_PATH: &str = "relativePath";
    pub(crate) const ENTRYPOINT_KIND: &str = "entrypointKind";
    pub(crate) const IMPORTED_BY_COUNT: &str = "importedByCount";
    pub(crate) const IMPORT_COUNT: &str = "importCount";
    pub(crate) const EXPORT_COUNT: &str = "exportCount";
}
