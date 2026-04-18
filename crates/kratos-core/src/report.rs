use std::path::Path;

use serde_json::{json, Value};

use crate::error::{KratosError, KratosResult};
use crate::model::{
    BrokenImportFinding, DeadExportFinding, DeletionCandidateFinding, EntrypointKind, ImportKind,
    ModuleRecord, OrphanFileFinding, OrphanKind, ReportV2, RouteEntrypointFinding, SummaryCounts,
    UnusedImportFinding, REPORT_V2,
};

pub fn validate_report_version(report: &ReportV2) -> KratosResult<()> {
    if report.version != REPORT_V2 {
        return Err(KratosError::InvalidReportVersion {
            expected: REPORT_V2,
            found: report.version,
        });
    }

    Ok(())
}

pub fn serialize_report_pretty(_report: &ReportV2) -> KratosResult<String> {
    validate_report_version(_report)?;
    let value = json!({
        "schemaVersion": _report.version,
        "generatedAt": _report.generated_at,
        "project": {
            "root": path_to_string(&_report.root),
            "configPath": _report
                .config_path
                .as_ref()
                .map(|path| Value::String(path_to_string(path)))
                .unwrap_or(Value::Null),
        },
        "summary": serialize_summary(&_report.summary),
        "findings": {
            "brokenImports": _report.findings.broken_imports.iter().map(serialize_broken_import).collect::<Vec<_>>(),
            "orphanFiles": _report.findings.orphan_files.iter().map(serialize_orphan_file).collect::<Vec<_>>(),
            "deadExports": _report.findings.dead_exports.iter().map(serialize_dead_export).collect::<Vec<_>>(),
            "unusedImports": _report.findings.unused_imports.iter().map(serialize_unused_import).collect::<Vec<_>>(),
            "routeEntrypoints": _report.findings.route_entrypoints.iter().map(serialize_route_entrypoint).collect::<Vec<_>>(),
            "deletionCandidates": _report.findings.deletion_candidates.iter().map(serialize_deletion_candidate).collect::<Vec<_>>(),
        },
        "graph": {
            "modules": _report.modules.iter().map(serialize_module).collect::<Vec<_>>(),
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

    let project = value.get("project");
    let root = project
        .and_then(|project| project.get("root"))
        .or_else(|| value.get("root"))
        .and_then(Value::as_str)
        .ok_or_else(|| KratosError::Json("Report is missing root".to_string()))?;
    let config_path = project
        .and_then(|project| project.get("configPath"))
        .and_then(Value::as_str)
        .map(Into::into);

    let (summary, finding_set, modules) = if version == REPORT_V2 {
        let summary =
            parse_required_summary(read_required_object(value.get("summary"), "summary")?)?;
        let findings = read_required_object(value.get("findings"), "findings")?;
        let modules = read_required_array(
            value.get("graph").and_then(|graph| graph.get("modules")),
            "graph.modules",
        )?;

        (
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

        (
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
        version,
        generated_at: value
            .get("generatedAt")
            .and_then(Value::as_str)
            .map(str::to_string),
        root: root.into(),
        config_path,
        summary,
        findings: finding_set,
        modules,
    })
}

pub fn format_summary_report(report: &ReportV2, report_path: &Path) -> KratosResult<String> {
    validate_report_version(report)?;
    let mut lines = vec![
        "Kratos report.".to_string(),
        String::new(),
        format!("Root: {}", path_to_string(&report.root)),
        format!("Files scanned: {}", report.summary.files_scanned),
        format!("Entrypoints: {}", report.summary.entrypoints),
        format!("Broken imports: {}", report.summary.broken_imports),
        format!("Orphan files: {}", report.summary.orphan_files),
        format!("Dead exports: {}", report.summary.dead_exports),
        format!("Unused imports: {}", report.summary.unused_imports),
        format!(
            "Deletion candidates: {}",
            report.summary.deletion_candidates
        ),
        String::new(),
        format!("Saved report: {}", path_to_string(report_path)),
    ];

    append_preview(
        &mut lines,
        "Broken imports",
        &report.findings.broken_imports,
        |item| format!("{} -> {}", path_to_string(&item.file), item.source),
    );
    append_preview(
        &mut lines,
        "Orphan files",
        &report.findings.orphan_files,
        |item| path_to_string(&item.file),
    );
    append_preview(
        &mut lines,
        "Dead exports",
        &report.findings.dead_exports,
        |item| format!("{}#{}", path_to_string(&item.file), item.export_name),
    );

    Ok(lines.join("\n"))
}

pub fn format_markdown_report(report: &ReportV2, report_path: &Path) -> KratosResult<String> {
    validate_report_version(report)?;

    let mut lines = vec![
        "# Kratos Report".to_string(),
        String::new(),
        format!(
            "- Generated: {}",
            report.generated_at.as_deref().unwrap_or_default()
        ),
        format!("- Root: {}", path_to_string(&report.root)),
        format!("- Report: {}", path_to_string(report_path)),
        String::new(),
        "## Summary".to_string(),
        String::new(),
        format!("- Files scanned: {}", report.summary.files_scanned),
        format!("- Entrypoints: {}", report.summary.entrypoints),
        format!("- Broken imports: {}", report.summary.broken_imports),
        format!("- Orphan files: {}", report.summary.orphan_files),
        format!("- Dead exports: {}", report.summary.dead_exports),
        format!("- Unused imports: {}", report.summary.unused_imports),
        format!(
            "- Deletion candidates: {}",
            report.summary.deletion_candidates
        ),
        String::new(),
    ];

    push_markdown_section(
        &mut lines,
        "Broken imports",
        &report.findings.broken_imports,
        |item| format!("{} -> `{}`", path_to_string(&item.file), item.source),
    );
    push_markdown_section(
        &mut lines,
        "Orphan files",
        &report.findings.orphan_files,
        |item| format!("{} ({})", path_to_string(&item.file), item.reason),
    );
    push_markdown_section(
        &mut lines,
        "Dead exports",
        &report.findings.dead_exports,
        |item| format!("{} -> `{}`", path_to_string(&item.file), item.export_name),
    );
    push_markdown_section(
        &mut lines,
        "Unused imports",
        &report.findings.unused_imports,
        |item| {
            format!(
                "{} -> `{}` from `{}`",
                path_to_string(&item.file),
                item.local,
                item.source
            )
        },
    );
    push_markdown_section(
        &mut lines,
        "Deletion candidates",
        &report.findings.deletion_candidates,
        |item| {
            format!(
                "{} ({}, confidence {})",
                path_to_string(&item.file),
                item.reason,
                item.confidence
            )
        },
    );

    Ok(lines.join("\n"))
}

fn serialize_summary(summary: &SummaryCounts) -> Value {
    json!({
        "filesScanned": summary.files_scanned,
        "entrypoints": summary.entrypoints,
        "brokenImports": summary.broken_imports,
        "orphanFiles": summary.orphan_files,
        "deadExports": summary.dead_exports,
        "unusedImports": summary.unused_imports,
        "routeEntrypoints": summary.route_entrypoints,
        "deletionCandidates": summary.deletion_candidates,
    })
}

fn serialize_broken_import(item: &BrokenImportFinding) -> Value {
    json!({
        "file": path_to_string(&item.file),
        "source": item.source,
        "kind": import_kind_to_string(&item.kind),
    })
}

fn serialize_orphan_file(item: &OrphanFileFinding) -> Value {
    json!({
        "file": path_to_string(&item.file),
        "kind": orphan_kind_to_string(&item.kind),
        "reason": item.reason,
        "confidence": round_confidence(item.confidence),
    })
}

fn serialize_dead_export(item: &DeadExportFinding) -> Value {
    json!({
        "file": path_to_string(&item.file),
        "exportName": item.export_name,
    })
}

fn serialize_unused_import(item: &UnusedImportFinding) -> Value {
    json!({
        "file": path_to_string(&item.file),
        "source": item.source,
        "local": item.local,
        "imported": item.imported,
    })
}

fn serialize_route_entrypoint(item: &RouteEntrypointFinding) -> Value {
    json!({
        "file": path_to_string(&item.file),
        "kind": entrypoint_kind_to_string(&item.kind),
    })
}

fn serialize_deletion_candidate(item: &DeletionCandidateFinding) -> Value {
    json!({
        "file": path_to_string(&item.file),
        "reason": item.reason,
        "confidence": round_confidence(item.confidence),
        "safe": item.safe,
    })
}

fn serialize_module(module: &ModuleRecord) -> Value {
    json!({
        "file": path_to_string(&module.file_path),
        "relativePath": module.relative_path,
        "entrypointKind": module.entrypoint_kind.as_ref().map(entrypoint_kind_to_string),
        "importedByCount": module.imported_by_count.max(module.imported_by.len()),
        "importCount": module.import_count.max(module.resolved_imports.len()),
        "exportCount": module.export_count.max(module.exports.len()),
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
                    .map(parse_import_kind)
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
            let kind = read_required_string(
                object,
                "kind",
                &format!("findings.brokenImports[{index}].kind"),
            )?;

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
                kind: parse_import_kind(kind),
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
                    .map(parse_orphan_kind)
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
            let kind = read_required_string(
                object,
                "kind",
                &format!("findings.orphanFiles[{index}].kind"),
            )?;

            Ok(OrphanFileFinding {
                file: read_required_string(
                    object,
                    "file",
                    &format!("findings.orphanFiles[{index}].file"),
                )?
                .into(),
                kind: parse_orphan_kind(kind),
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
            Some(DeadExportFinding {
                file: item.get("file")?.as_str()?.into(),
                export_name: item.get("exportName")?.as_str()?.to_string(),
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

fn append_preview<T>(
    lines: &mut Vec<String>,
    label: &str,
    items: &[T],
    render: impl Fn(&T) -> String,
) {
    if items.is_empty() {
        return;
    }

    lines.push(String::new());
    lines.push(format!("{label}:"));

    for item in items.iter().take(5) {
        lines.push(format!("- {}", render(item)));
    }

    if items.len() > 5 {
        lines.push(format!("- ...and {} more", items.len() - 5));
    }
}

fn push_markdown_section<T>(
    lines: &mut Vec<String>,
    title: &str,
    items: &[T],
    render: impl Fn(&T) -> String,
) {
    lines.push(format!("## {title}"));
    lines.push(String::new());

    if items.is_empty() {
        lines.push("- None".to_string());
        lines.push(String::new());
        return;
    }

    for item in items {
        lines.push(format!("- {}", render(item)));
    }

    lines.push(String::new());
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn entrypoint_kind_to_string(kind: &EntrypointKind) -> &'static str {
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

fn parse_import_kind(value: &str) -> ImportKind {
    match value {
        "static" => ImportKind::Static,
        "side-effect" => ImportKind::SideEffect,
        "reexport" => ImportKind::Reexport,
        "reexport-all" => ImportKind::ReexportAll,
        "reexport-namespace" => ImportKind::ReexportNamespace,
        "require" => ImportKind::Require,
        "dynamic" => ImportKind::Dynamic,
        _ => ImportKind::Unknown,
    }
}

fn parse_orphan_kind(value: &str) -> OrphanKind {
    match value {
        "orphan-component" => OrphanKind::Component,
        "orphan-route-module" => OrphanKind::RouteModule,
        _ => OrphanKind::Module,
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
