use std::path::Path;

use serde_json::{json, Value};

use crate::error::{KratosError, KratosResult};
use crate::model::{
    BrokenImportFinding, CleanCandidateFingerprint, CleanSafetyManifest, DeadExportFinding,
    DeletionCandidateFinding, EntrypointKind, ExportKind, ImportKind, ModuleRecord,
    OrphanFileFinding, OrphanKind, ReportV2, RouteEntrypointFinding, SummaryCounts,
    UnusedImportFinding, REPORT_V2,
};
use crate::report_contract::{
    clean_safety, finding_field, findings, graph, module, project, summary, top_level,
    REPORT_SCHEMA_VERSION,
};

pub fn validate_report_version(report: &ReportV2) -> KratosResult<()> {
    if report.version != REPORT_SCHEMA_VERSION {
        return Err(KratosError::InvalidReportVersion {
            expected: REPORT_SCHEMA_VERSION,
            found: report.version,
        });
    }

    Ok(())
}

pub fn serialize_report_pretty(_report: &ReportV2) -> KratosResult<String> {
    validate_report_version(_report)?;
    let value = json!({
        top_level::SCHEMA_VERSION: _report.version,
        top_level::GENERATED_AT: _report.generated_at,
        top_level::PROJECT: {
            project::ROOT: path_to_string(&_report.root),
            project::CONFIG_PATH: _report
                .config_path
                .as_ref()
                .map(|path| Value::String(path_to_string(path)))
                .unwrap_or(Value::Null),
        },
        top_level::SUMMARY: serialize_summary(&_report.summary),
        top_level::FINDINGS: {
            findings::BROKEN_IMPORTS: _report.findings.broken_imports.iter().map(serialize_broken_import).collect::<Vec<_>>(),
            findings::ORPHAN_FILES: _report.findings.orphan_files.iter().map(serialize_orphan_file).collect::<Vec<_>>(),
            findings::DEAD_EXPORTS: _report.findings.dead_exports.iter().map(serialize_dead_export).collect::<Vec<_>>(),
            findings::UNUSED_IMPORTS: _report.findings.unused_imports.iter().map(serialize_unused_import).collect::<Vec<_>>(),
            findings::ROUTE_ENTRYPOINTS: _report.findings.route_entrypoints.iter().map(serialize_route_entrypoint).collect::<Vec<_>>(),
            findings::DELETION_CANDIDATES: _report.findings.deletion_candidates.iter().map(serialize_deletion_candidate).collect::<Vec<_>>(),
        },
        top_level::CLEAN_SAFETY: serialize_clean_safety(&_report.clean_safety),
        top_level::GRAPH: {
            graph::MODULES: _report.modules.iter().map(serialize_module).collect::<Vec<_>>(),
        },
    });

    serde_json::to_string_pretty(&value).map_err(|error| KratosError::Json(error.to_string()))
}

pub fn parse_report_json(raw: &str) -> KratosResult<ReportV2> {
    let value: Value =
        serde_json::from_str(raw).map_err(|error| KratosError::Json(error.to_string()))?;

    let version = value
        .get("schemaVersion")
        .or_else(|| value.get("version"))
        .and_then(Value::as_u64)
        .ok_or_else(|| KratosError::Json("Report is missing schemaVersion/version".to_string()))?
        as u32;

    let generated_at =
        read_optional_string(Some(&value), "generatedAt", "generatedAt")?.map(str::to_string);
    let clean_safety = if version >= REPORT_SCHEMA_VERSION {
        parse_required_clean_safety(value.get("cleanSafety"))?
    } else {
        CleanSafetyManifest::default()
    };

    let (root, config_path, summary, finding_set, modules) = if version >= REPORT_V2 {
        let project = read_required_object(value.get("project"), "project")?;
        let summary =
            parse_required_summary(read_required_object(value.get("summary"), "summary")?)?;
        let findings = read_required_object(value.get("findings"), "findings")?;
        let modules = read_required_array(
            value.get("graph").and_then(|graph| graph.get("modules")),
            "graph.modules",
        )?;

        (
            read_required_string(project, "root", "project.root")?,
            read_optional_string(value.get("project"), "configPath", "project.configPath")?
                .map(Into::into),
            summary,
            crate::model::FindingSet {
                broken_imports: parse_required_broken_imports(read_required_array(
                    findings.get("brokenImports"),
                    "findings.brokenImports",
                )?)?,
                orphan_files: parse_required_orphan_files(read_required_array(
                    findings.get("orphanFiles"),
                    "findings.orphanFiles",
                )?)?,
                dead_exports: parse_required_dead_exports(read_required_array(
                    findings.get("deadExports"),
                    "findings.deadExports",
                )?)?,
                unused_imports: parse_required_unused_imports(read_required_array(
                    findings.get("unusedImports"),
                    "findings.unusedImports",
                )?)?,
                route_entrypoints: parse_required_route_entrypoints(read_required_array(
                    findings.get("routeEntrypoints"),
                    "findings.routeEntrypoints",
                )?)?,
                deletion_candidates: parse_required_deletion_candidates(read_required_array(
                    findings.get("deletionCandidates"),
                    "findings.deletionCandidates",
                )?)?,
            },
            parse_required_modules(modules)?,
        )
    } else {
        let findings = value.get("findings");
        let modules = value
            .get("graph")
            .and_then(|graph| graph.get("modules"))
            .or_else(|| value.get("modules"));
        let project = value.get("project");

        (
            project
                .and_then(|project| project.get("root"))
                .or_else(|| value.get("root"))
                .and_then(Value::as_str)
                .ok_or_else(|| KratosError::Json("Report is missing root".to_string()))?,
            read_optional_string(project, "configPath", "project.configPath")?.map(Into::into),
            parse_summary(value.get("summary")),
            crate::model::FindingSet {
                broken_imports: parse_broken_imports(
                    findings.and_then(|item| item.get("brokenImports")),
                ),
                orphan_files: parse_orphan_files(findings.and_then(|item| item.get("orphanFiles"))),
                dead_exports: parse_dead_exports(findings.and_then(|item| item.get("deadExports"))),
                unused_imports: parse_unused_imports(
                    findings.and_then(|item| item.get("unusedImports")),
                ),
                route_entrypoints: parse_route_entrypoints(
                    findings.and_then(|item| item.get("routeEntrypoints")),
                ),
                deletion_candidates: parse_deletion_candidates(
                    findings.and_then(|item| item.get("deletionCandidates")),
                ),
            },
            parse_modules(modules),
        )
    };

    Ok(ReportV2 {
        version: if version < REPORT_V2 {
            REPORT_V2
        } else {
            version
        },
        generated_at,
        root: root.into(),
        config_path,
        summary,
        findings: finding_set,
        clean_safety,
        modules,
    })
}

pub fn format_summary_report(report: &ReportV2, report_path: &Path) -> KratosResult<String> {
    crate::report_format::format_summary_report(report, report_path, "Kratos report.")
}

pub fn format_markdown_report(report: &ReportV2, report_path: &Path) -> KratosResult<String> {
    crate::report_format::format_markdown_report(report, report_path)
}

fn serialize_summary(summary_counts: &SummaryCounts) -> Value {
    let mut summary_value = serde_json::Map::new();
    summary_value.insert(
        summary::FILES_SCANNED.to_string(),
        json!(summary_counts.files_scanned),
    );
    summary_value.insert(
        summary::ENTRYPOINTS.to_string(),
        json!(summary_counts.entrypoints),
    );
    summary_value.insert(
        summary::BROKEN_IMPORTS.to_string(),
        json!(summary_counts.broken_imports),
    );
    summary_value.insert(
        summary::ORPHAN_FILES.to_string(),
        json!(summary_counts.orphan_files),
    );
    summary_value.insert(
        summary::DEAD_EXPORTS.to_string(),
        json!(summary_counts.dead_exports),
    );
    summary_value.insert(
        summary::UNUSED_IMPORTS.to_string(),
        json!(summary_counts.unused_imports),
    );
    summary_value.insert(
        summary::ROUTE_ENTRYPOINTS.to_string(),
        json!(summary_counts.route_entrypoints),
    );
    summary_value.insert(
        summary::DELETION_CANDIDATES.to_string(),
        json!(summary_counts.deletion_candidates),
    );

    if summary_counts.suppressed_findings > 0 {
        summary_value.insert(
            summary::SUPPRESSED_FINDINGS.to_string(),
            json!(summary_counts.suppressed_findings),
        );
    }

    Value::Object(summary_value)
}

fn serialize_broken_import(item: &BrokenImportFinding) -> Value {
    json!({
        finding_field::FILE: path_to_string(&item.file),
        finding_field::SOURCE: item.source,
        finding_field::KIND: import_kind_to_string(&item.kind),
    })
}

fn serialize_orphan_file(item: &OrphanFileFinding) -> Value {
    json!({
        finding_field::FILE: path_to_string(&item.file),
        finding_field::KIND: orphan_kind_to_string(&item.kind),
        finding_field::REASON: item.reason,
        finding_field::CONFIDENCE: round_confidence(item.confidence),
    })
}

fn serialize_dead_export(item: &DeadExportFinding) -> Value {
    json!({
        finding_field::FILE: path_to_string(&item.file),
        finding_field::EXPORT_NAME: item.export_name,
        finding_field::EXPORT_KIND: export_kind_to_string(&item.export_kind),
        finding_field::REASON: item.reason,
        finding_field::CONFIDENCE: round_confidence(item.confidence),
        finding_field::IMPORTED_BY_COUNT: item.imported_by_count,
        finding_field::USED_EXPORT_NAMES: item.used_export_names,
        finding_field::HAS_NAMESPACE_OR_UNKNOWN_USAGE: item.has_namespace_or_unknown_usage,
    })
}

fn serialize_unused_import(item: &UnusedImportFinding) -> Value {
    json!({
        finding_field::FILE: path_to_string(&item.file),
        finding_field::SOURCE: item.source,
        finding_field::LOCAL: item.local,
        finding_field::IMPORTED: item.imported,
    })
}

fn serialize_route_entrypoint(item: &RouteEntrypointFinding) -> Value {
    json!({
        finding_field::FILE: path_to_string(&item.file),
        finding_field::KIND: entrypoint_kind_to_string(&item.kind),
    })
}

fn serialize_deletion_candidate(item: &DeletionCandidateFinding) -> Value {
    json!({
        finding_field::FILE: path_to_string(&item.file),
        finding_field::REASON: item.reason,
        finding_field::CONFIDENCE: round_confidence(item.confidence),
        finding_field::SAFE: item.safe,
    })
}

fn serialize_clean_safety(manifest: &CleanSafetyManifest) -> Value {
    json!({
        clean_safety::FINGERPRINT_ALGORITHM: manifest.fingerprint_algorithm,
        clean_safety::CANDIDATES: manifest
            .candidates
            .iter()
            .map(|candidate| {
                json!({
                    clean_safety::FILE: path_to_string(&candidate.file),
                    clean_safety::FINGERPRINT: candidate.fingerprint,
                    clean_safety::IDENTITY: candidate.identity,
                    clean_safety::PARENT_IDENTITY: candidate.parent_identity,
                })
            })
            .collect::<Vec<_>>(),
    })
}

fn serialize_module(module_record: &ModuleRecord) -> Value {
    json!({
        module::FILE: path_to_string(&module_record.file_path),
        module::RELATIVE_PATH: module_record.relative_path,
        module::ENTRYPOINT_KIND: module_record.entrypoint_kind.as_ref().map(entrypoint_kind_to_string),
        module::IMPORTED_BY_COUNT: module_record.imported_by_count.max(module_record.imported_by.len()),
        module::IMPORT_COUNT: module_record.import_count.max(module_record.resolved_imports.len()),
        module::EXPORT_COUNT: module_record.export_count.max(module_record.exports.len()),
    })
}

fn parse_summary(value: Option<&Value>) -> SummaryCounts {
    let Some(value) = value else {
        return SummaryCounts::default();
    };

    SummaryCounts {
        files_scanned: read_usize(value, "filesScanned"),
        entrypoints: read_usize(value, "entrypoints"),
        broken_imports: read_usize(value, "brokenImports"),
        orphan_files: read_usize(value, "orphanFiles"),
        dead_exports: read_usize(value, "deadExports"),
        unused_imports: read_usize(value, "unusedImports"),
        route_entrypoints: read_usize(value, "routeEntrypoints"),
        deletion_candidates: read_usize(value, "deletionCandidates"),
        suppressed_findings: read_usize(value, "suppressedFindings"),
    }
}

fn parse_required_summary(value: &serde_json::Map<String, Value>) -> KratosResult<SummaryCounts> {
    Ok(SummaryCounts {
        files_scanned: read_required_usize(value, "filesScanned", "summary.filesScanned")?,
        entrypoints: read_required_usize(value, "entrypoints", "summary.entrypoints")?,
        broken_imports: read_required_usize(value, "brokenImports", "summary.brokenImports")?,
        orphan_files: read_required_usize(value, "orphanFiles", "summary.orphanFiles")?,
        dead_exports: read_required_usize(value, "deadExports", "summary.deadExports")?,
        unused_imports: read_required_usize(value, "unusedImports", "summary.unusedImports")?,
        route_entrypoints: read_required_usize(
            value,
            "routeEntrypoints",
            "summary.routeEntrypoints",
        )?,
        deletion_candidates: read_required_usize(
            value,
            "deletionCandidates",
            "summary.deletionCandidates",
        )?,
        suppressed_findings: read_optional_usize(
            value,
            "suppressedFindings",
            "summary.suppressedFindings",
        )?,
    })
}

fn parse_broken_imports(value: Option<&Value>) -> Vec<BrokenImportFinding> {
    read_array(value)
        .iter()
        .filter_map(|item| {
            Some(BrokenImportFinding {
                file: item.get("file")?.as_str()?.into(),
                source: item.get("source")?.as_str()?.to_string(),
                kind: item
                    .get("kind")
                    .and_then(Value::as_str)
                    .and_then(parse_import_kind)
                    .unwrap_or(ImportKind::Unknown),
            })
        })
        .collect()
}

fn parse_required_broken_imports(values: &[Value]) -> KratosResult<Vec<BrokenImportFinding>> {
    values
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let object =
                read_required_object(Some(item), &format!("findings.brokenImports[{index}]"))?;

            Ok(BrokenImportFinding {
                file: read_required_string(
                    object,
                    "file",
                    &format!("findings.brokenImports[{index}].file"),
                )?
                .into(),
                source: read_required_string(
                    object,
                    "source",
                    &format!("findings.brokenImports[{index}].source"),
                )?
                .to_string(),
                kind: read_required_import_kind(
                    object,
                    "kind",
                    &format!("findings.brokenImports[{index}].kind"),
                )?,
            })
        })
        .collect()
}

fn parse_orphan_files(value: Option<&Value>) -> Vec<OrphanFileFinding> {
    read_array(value)
        .iter()
        .filter_map(|item| {
            Some(OrphanFileFinding {
                file: item.get("file")?.as_str()?.into(),
                kind: item
                    .get("kind")
                    .and_then(Value::as_str)
                    .and_then(parse_orphan_kind)
                    .unwrap_or(OrphanKind::Module),
                reason: item
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                confidence: item
                    .get("confidence")
                    .and_then(Value::as_f64)
                    .unwrap_or_default() as f32,
            })
        })
        .collect()
}

fn parse_required_orphan_files(values: &[Value]) -> KratosResult<Vec<OrphanFileFinding>> {
    values
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let object =
                read_required_object(Some(item), &format!("findings.orphanFiles[{index}]"))?;

            Ok(OrphanFileFinding {
                file: read_required_string(
                    object,
                    "file",
                    &format!("findings.orphanFiles[{index}].file"),
                )?
                .into(),
                kind: read_required_orphan_kind(
                    object,
                    "kind",
                    &format!("findings.orphanFiles[{index}].kind"),
                )?,
                reason: read_required_string(
                    object,
                    "reason",
                    &format!("findings.orphanFiles[{index}].reason"),
                )?
                .to_string(),
                confidence: read_required_f64(
                    object,
                    "confidence",
                    &format!("findings.orphanFiles[{index}].confidence"),
                )? as f32,
            })
        })
        .collect()
}

fn parse_dead_exports(value: Option<&Value>) -> Vec<DeadExportFinding> {
    read_array(value)
        .iter()
        .filter_map(|item| {
            let object = item.as_object()?;
            Some(DeadExportFinding {
                file: object.get("file")?.as_str()?.into(),
                export_name: object.get("exportName")?.as_str()?.to_string(),
                export_kind: object
                    .get("exportKind")
                    .and_then(Value::as_str)
                    .and_then(parse_export_kind)
                    .unwrap_or(ExportKind::Unknown),
                reason: object
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                confidence: object
                    .get("confidence")
                    .and_then(Value::as_f64)
                    .unwrap_or_default() as f32,
                imported_by_count: object
                    .get("importedByCount")
                    .and_then(Value::as_u64)
                    .unwrap_or_default() as usize,
                used_export_names: object
                    .get("usedExportNames")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
                has_namespace_or_unknown_usage: object
                    .get("hasNamespaceOrUnknownUsage")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
}

fn parse_required_dead_exports(values: &[Value]) -> KratosResult<Vec<DeadExportFinding>> {
    values
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let object =
                read_required_object(Some(item), &format!("findings.deadExports[{index}]"))?;

            Ok(DeadExportFinding {
                file: read_required_string(
                    object,
                    "file",
                    &format!("findings.deadExports[{index}].file"),
                )?
                .into(),
                export_name: read_required_string(
                    object,
                    "exportName",
                    &format!("findings.deadExports[{index}].exportName"),
                )?
                .to_string(),
                export_kind: read_optional_export_kind(
                    object,
                    "exportKind",
                    &format!("findings.deadExports[{index}].exportKind"),
                )?,
                reason: read_optional_string_owned(
                    object,
                    "reason",
                    &format!("findings.deadExports[{index}].reason"),
                )?,
                confidence: read_optional_f64(
                    object,
                    "confidence",
                    &format!("findings.deadExports[{index}].confidence"),
                )? as f32,
                imported_by_count: read_optional_usize(
                    object,
                    "importedByCount",
                    &format!("findings.deadExports[{index}].importedByCount"),
                )?,
                used_export_names: read_optional_string_vec(
                    object,
                    "usedExportNames",
                    &format!("findings.deadExports[{index}].usedExportNames"),
                )?,
                has_namespace_or_unknown_usage: read_optional_bool(
                    object,
                    "hasNamespaceOrUnknownUsage",
                    &format!("findings.deadExports[{index}].hasNamespaceOrUnknownUsage"),
                )?,
            })
        })
        .collect()
}

fn parse_unused_imports(value: Option<&Value>) -> Vec<UnusedImportFinding> {
    read_array(value)
        .iter()
        .filter_map(|item| {
            Some(UnusedImportFinding {
                file: item.get("file")?.as_str()?.into(),
                source: item.get("source")?.as_str()?.to_string(),
                local: item.get("local")?.as_str()?.to_string(),
                imported: item.get("imported")?.as_str()?.to_string(),
            })
        })
        .collect()
}

fn parse_required_unused_imports(values: &[Value]) -> KratosResult<Vec<UnusedImportFinding>> {
    values
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let object =
                read_required_object(Some(item), &format!("findings.unusedImports[{index}]"))?;

            Ok(UnusedImportFinding {
                file: read_required_string(
                    object,
                    "file",
                    &format!("findings.unusedImports[{index}].file"),
                )?
                .into(),
                source: read_required_string(
                    object,
                    "source",
                    &format!("findings.unusedImports[{index}].source"),
                )?
                .to_string(),
                local: read_required_string(
                    object,
                    "local",
                    &format!("findings.unusedImports[{index}].local"),
                )?
                .to_string(),
                imported: read_required_string(
                    object,
                    "imported",
                    &format!("findings.unusedImports[{index}].imported"),
                )?
                .to_string(),
            })
        })
        .collect()
}

fn parse_route_entrypoints(value: Option<&Value>) -> Vec<RouteEntrypointFinding> {
    read_array(value)
        .iter()
        .filter_map(|item| {
            let kind = item
                .get("kind")
                .and_then(Value::as_str)
                .and_then(parse_entrypoint_kind)?;

            Some(RouteEntrypointFinding {
                file: item.get("file")?.as_str()?.into(),
                kind,
            })
        })
        .collect()
}

fn parse_required_route_entrypoints(values: &[Value]) -> KratosResult<Vec<RouteEntrypointFinding>> {
    values
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let object =
                read_required_object(Some(item), &format!("findings.routeEntrypoints[{index}]"))?;
            let kind = read_required_entrypoint_kind(
                object,
                "kind",
                &format!("findings.routeEntrypoints[{index}].kind"),
            )?;

            Ok(RouteEntrypointFinding {
                file: read_required_string(
                    object,
                    "file",
                    &format!("findings.routeEntrypoints[{index}].file"),
                )?
                .into(),
                kind,
            })
        })
        .collect()
}

fn parse_deletion_candidates(value: Option<&Value>) -> Vec<DeletionCandidateFinding> {
    read_array(value)
        .iter()
        .filter_map(|item| {
            Some(DeletionCandidateFinding {
                file: item.get("file")?.as_str()?.into(),
                reason: item
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                confidence: item
                    .get("confidence")
                    .and_then(Value::as_f64)
                    .unwrap_or_default() as f32,
                safe: item.get("safe").and_then(Value::as_bool).unwrap_or(false),
            })
        })
        .collect()
}

fn parse_required_deletion_candidates(
    values: &[Value],
) -> KratosResult<Vec<DeletionCandidateFinding>> {
    values
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let object =
                read_required_object(Some(item), &format!("findings.deletionCandidates[{index}]"))?;

            Ok(DeletionCandidateFinding {
                file: read_required_string(
                    object,
                    "file",
                    &format!("findings.deletionCandidates[{index}].file"),
                )?
                .into(),
                reason: read_required_string(
                    object,
                    "reason",
                    &format!("findings.deletionCandidates[{index}].reason"),
                )?
                .to_string(),
                confidence: read_required_f64(
                    object,
                    "confidence",
                    &format!("findings.deletionCandidates[{index}].confidence"),
                )? as f32,
                safe: read_required_bool(
                    object,
                    "safe",
                    &format!("findings.deletionCandidates[{index}].safe"),
                )?,
            })
        })
        .collect()
}

fn parse_required_clean_safety(value: Option<&Value>) -> KratosResult<CleanSafetyManifest> {
    let object = read_required_object(value, "cleanSafety")?;
    let fingerprint_algorithm = read_required_string(
        object,
        "fingerprintAlgorithm",
        "cleanSafety.fingerprintAlgorithm",
    )?
    .to_string();
    let candidates = read_required_array(object.get("candidates"), "cleanSafety.candidates")?
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let path = format!("cleanSafety.candidates[{index}]");
            let candidate = read_required_object(Some(item), &path)?;
            let file = read_required_string(candidate, "file", &format!("{path}.file"))?.into();
            let fingerprint = match candidate.get("fingerprint") {
                Some(Value::Null) => None,
                Some(Value::String(value)) => Some(value.clone()),
                Some(_) => {
                    return Err(KratosError::Json(format!(
                        "{path}.fingerprint must be a string or null"
                    )))
                }
                None => return Err(KratosError::Json(format!("{path}.fingerprint is required"))),
            };
            let identity = match candidate.get("identity") {
                Some(Value::Null) => None,
                Some(Value::String(value)) => Some(value.clone()),
                Some(_) => {
                    return Err(KratosError::Json(format!(
                        "{path}.identity must be a string or null"
                    )))
                }
                None => return Err(KratosError::Json(format!("{path}.identity is required"))),
            };
            let parent_identity = match candidate.get("parentIdentity") {
                Some(Value::Null) => None,
                Some(Value::String(value)) => Some(value.clone()),
                Some(_) => {
                    return Err(KratosError::Json(format!(
                        "{path}.parentIdentity must be a string or null"
                    )))
                }
                None => {
                    return Err(KratosError::Json(format!(
                        "{path}.parentIdentity is required"
                    )))
                }
            };

            Ok(CleanCandidateFingerprint {
                file,
                fingerprint,
                identity,
                parent_identity,
            })
        })
        .collect::<KratosResult<Vec<_>>>()?;

    Ok(CleanSafetyManifest {
        fingerprint_algorithm,
        candidates,
    })
}

fn parse_modules(value: Option<&Value>) -> Vec<ModuleRecord> {
    read_array(value)
        .iter()
        .filter_map(|item| {
            Some(ModuleRecord {
                file_path: item.get("file")?.as_str()?.into(),
                relative_path: item
                    .get("relativePath")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                entrypoint_kind: item
                    .get("entrypointKind")
                    .and_then(Value::as_str)
                    .and_then(parse_entrypoint_kind),
                imports: Vec::new(),
                exports: Vec::new(),
                unused_imports: Vec::new(),
                resolved_imports: Vec::new(),
                importers: Vec::new(),
                imported_by: Vec::new(),
                imported_by_count: item
                    .get("importedByCount")
                    .and_then(Value::as_u64)
                    .unwrap_or_default() as usize,
                import_count: item
                    .get("importCount")
                    .and_then(Value::as_u64)
                    .unwrap_or_default() as usize,
                export_count: item
                    .get("exportCount")
                    .and_then(Value::as_u64)
                    .unwrap_or_default() as usize,
            })
        })
        .collect()
}

fn parse_required_modules(values: &[Value]) -> KratosResult<Vec<ModuleRecord>> {
    values
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let object = read_required_object(Some(item), &format!("graph.modules[{index}]"))?;

            Ok(ModuleRecord {
                file_path: read_required_string(
                    object,
                    "file",
                    &format!("graph.modules[{index}].file"),
                )?
                .into(),
                relative_path: read_required_string(
                    object,
                    "relativePath",
                    &format!("graph.modules[{index}].relativePath"),
                )?
                .to_string(),
                entrypoint_kind: object
                    .get("entrypointKind")
                    .map(|_| {
                        read_optional_entrypoint_kind(
                            object,
                            "entrypointKind",
                            &format!("graph.modules[{index}].entrypointKind"),
                        )
                    })
                    .transpose()?
                    .flatten(),
                imports: Vec::new(),
                exports: Vec::new(),
                unused_imports: Vec::new(),
                resolved_imports: Vec::new(),
                importers: Vec::new(),
                imported_by: Vec::new(),
                imported_by_count: read_required_usize(
                    object,
                    "importedByCount",
                    &format!("graph.modules[{index}].importedByCount"),
                )?,
                import_count: read_required_usize(
                    object,
                    "importCount",
                    &format!("graph.modules[{index}].importCount"),
                )?,
                export_count: read_required_usize(
                    object,
                    "exportCount",
                    &format!("graph.modules[{index}].exportCount"),
                )?,
            })
        })
        .collect()
}

pub(crate) fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub(crate) fn entrypoint_kind_to_string(kind: &EntrypointKind) -> &'static str {
    match kind {
        EntrypointKind::UserEntry => "user-entry",
        EntrypointKind::PackageEntry => "package-entry",
        EntrypointKind::NextAppRoute => "next-app-route",
        EntrypointKind::NextPagesRoute => "next-pages-route",
        EntrypointKind::AppEntry => "app-entry",
        EntrypointKind::ToolingEntry => "tooling-entry",
        EntrypointKind::FrameworkEntry => "framework-entry",
    }
}

pub(crate) fn export_kind_to_string(kind: &ExportKind) -> &'static str {
    match kind {
        ExportKind::Default => "default",
        ExportKind::Named => "named",
        ExportKind::Reexport => "reexport",
        ExportKind::ReexportAll => "reexport-all",
        ExportKind::ReexportNamespace => "reexport-namespace",
        ExportKind::Unknown => "unknown",
    }
}

fn import_kind_to_string(kind: &ImportKind) -> &'static str {
    match kind {
        ImportKind::Static => "static",
        ImportKind::SideEffect => "side-effect",
        ImportKind::Reexport => "reexport",
        ImportKind::ReexportAll => "reexport-all",
        ImportKind::ReexportNamespace => "reexport-namespace",
        ImportKind::Require => "require",
        ImportKind::Dynamic => "dynamic",
        ImportKind::Unknown => "unknown",
    }
}

fn orphan_kind_to_string(kind: &OrphanKind) -> &'static str {
    match kind {
        OrphanKind::Module => "orphan-module",
        OrphanKind::Component => "orphan-component",
        OrphanKind::RouteModule => "orphan-route-module",
    }
}

fn parse_entrypoint_kind(value: &str) -> Option<EntrypointKind> {
    match value {
        "user-entry" => Some(EntrypointKind::UserEntry),
        "package-entry" => Some(EntrypointKind::PackageEntry),
        "next-app-route" => Some(EntrypointKind::NextAppRoute),
        "next-pages-route" => Some(EntrypointKind::NextPagesRoute),
        "app-entry" => Some(EntrypointKind::AppEntry),
        "tooling-entry" => Some(EntrypointKind::ToolingEntry),
        "framework-entry" => Some(EntrypointKind::FrameworkEntry),
        _ => None,
    }
}

fn parse_import_kind(value: &str) -> Option<ImportKind> {
    match value {
        "static" => Some(ImportKind::Static),
        "side-effect" => Some(ImportKind::SideEffect),
        "reexport" => Some(ImportKind::Reexport),
        "reexport-all" => Some(ImportKind::ReexportAll),
        "reexport-namespace" => Some(ImportKind::ReexportNamespace),
        "require" => Some(ImportKind::Require),
        "dynamic" => Some(ImportKind::Dynamic),
        "unknown" => Some(ImportKind::Unknown),
        _ => None,
    }
}

fn parse_export_kind(value: &str) -> Option<ExportKind> {
    match value {
        "default" => Some(ExportKind::Default),
        "named" => Some(ExportKind::Named),
        "reexport" => Some(ExportKind::Reexport),
        "reexport-all" => Some(ExportKind::ReexportAll),
        "reexport-namespace" => Some(ExportKind::ReexportNamespace),
        "unknown" => Some(ExportKind::Unknown),
        _ => None,
    }
}

fn parse_orphan_kind(value: &str) -> Option<OrphanKind> {
    match value {
        "orphan-component" => Some(OrphanKind::Component),
        "orphan-route-module" => Some(OrphanKind::RouteModule),
        "orphan-module" => Some(OrphanKind::Module),
        _ => None,
    }
}

fn read_array(value: Option<&Value>) -> &[Value] {
    value
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn read_usize(value: &Value, key: &str) -> usize {
    value.get(key).and_then(Value::as_u64).unwrap_or_default() as usize
}

fn read_required_object<'a>(
    value: Option<&'a Value>,
    path: &str,
) -> KratosResult<&'a serde_json::Map<String, Value>> {
    value
        .and_then(Value::as_object)
        .ok_or_else(|| KratosError::Json(format!("Report is missing required object `{path}`")))
}

fn read_required_array<'a>(value: Option<&'a Value>, path: &str) -> KratosResult<&'a [Value]> {
    value
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| KratosError::Json(format!("Report is missing required array `{path}`")))
}

fn read_required_string<'a>(
    value: &'a serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> KratosResult<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| KratosError::Json(format!("Report is missing required string `{path}`")))
}

fn read_optional_string<'a>(
    value: Option<&'a Value>,
    key: &str,
    path: &str,
) -> KratosResult<Option<&'a str>> {
    let Some(object) = value.and_then(Value::as_object) else {
        return Ok(None);
    };

    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(string)) => Ok(Some(string)),
        Some(_) => Err(KratosError::Json(format!(
            "Report has invalid string field `{path}`"
        ))),
    }
}

fn read_required_usize(
    value: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> KratosResult<usize> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .map(|number| number as usize)
        .ok_or_else(|| KratosError::Json(format!("Report is missing required number `{path}`")))
}

fn read_optional_usize(
    value: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> KratosResult<usize> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(0),
        Some(Value::Number(number)) => number
            .as_u64()
            .map(|number| number as usize)
            .ok_or_else(|| KratosError::Json(format!("Report has invalid number `{path}`"))),
        Some(_) => Err(KratosError::Json(format!(
            "Report has invalid number `{path}`"
        ))),
    }
}

fn read_optional_f64(
    value: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> KratosResult<f64> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(0.0),
        Some(Value::Number(number)) => number
            .as_f64()
            .ok_or_else(|| KratosError::Json(format!("Report has invalid number `{path}`"))),
        Some(_) => Err(KratosError::Json(format!(
            "Report has invalid number `{path}`"
        ))),
    }
}

fn read_optional_bool(
    value: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> KratosResult<bool> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(KratosError::Json(format!(
            "Report has invalid boolean `{path}`"
        ))),
    }
}

fn read_optional_string_owned(
    value: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> KratosResult<String> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(value)) => Ok(value.clone()),
        Some(_) => Err(KratosError::Json(format!(
            "Report has invalid string field `{path}`"
        ))),
    }
}

fn read_optional_string_vec(
    value: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> KratosResult<Vec<String>> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                item.as_str().map(str::to_string).ok_or_else(|| {
                    KratosError::Json(format!("Report has invalid string `{path}[{index}]`"))
                })
            })
            .collect(),
        Some(_) => Err(KratosError::Json(format!(
            "Report has invalid array `{path}`"
        ))),
    }
}

fn read_optional_export_kind(
    value: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> KratosResult<ExportKind> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(ExportKind::Unknown),
        Some(Value::String(raw)) => parse_export_kind(raw)
            .ok_or_else(|| KratosError::Json(format!("Report has invalid export kind `{path}`"))),
        Some(_) => Err(KratosError::Json(format!(
            "Report has invalid export kind `{path}`"
        ))),
    }
}

fn read_required_f64(
    value: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> KratosResult<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| KratosError::Json(format!("Report is missing required number `{path}`")))
}

fn read_required_bool(
    value: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> KratosResult<bool> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| KratosError::Json(format!("Report is missing required boolean `{path}`")))
}

fn read_required_entrypoint_kind(
    value: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> KratosResult<EntrypointKind> {
    let raw = read_required_string(value, key, path)?;
    parse_entrypoint_kind(raw)
        .ok_or_else(|| KratosError::Json(format!("Report has invalid entrypoint kind `{path}`")))
}

fn read_required_import_kind(
    value: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> KratosResult<ImportKind> {
    let raw = read_required_string(value, key, path)?;
    parse_import_kind(raw)
        .ok_or_else(|| KratosError::Json(format!("Report has invalid import kind `{path}`")))
}

fn read_required_orphan_kind(
    value: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> KratosResult<OrphanKind> {
    let raw = read_required_string(value, key, path)?;
    parse_orphan_kind(raw)
        .ok_or_else(|| KratosError::Json(format!("Report has invalid orphan kind `{path}`")))
}

fn read_optional_entrypoint_kind(
    value: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> KratosResult<Option<EntrypointKind>> {
    match value.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(Value::String(raw)) => parse_entrypoint_kind(raw).map(Some).ok_or_else(|| {
            KratosError::Json(format!("Report has invalid entrypoint kind `{path}`"))
        }),
        Some(_) => Err(KratosError::Json(format!(
            "Report is missing required string `{path}`"
        ))),
    }
}

fn round_confidence(value: f32) -> f64 {
    ((value as f64) * 100.0).round() / 100.0
}
