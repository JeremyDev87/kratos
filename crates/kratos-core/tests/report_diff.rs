use std::path::PathBuf;

use kratos_core::model::{
    BrokenImportFinding, DeadExportFinding, DeletionCandidateFinding, EntrypointKind, FindingSet,
    ImportKind, ModuleRecord, OrphanFileFinding, OrphanKind, ReportV2, RouteEntrypointFinding,
    SummaryCounts, UnusedImportFinding,
};
use kratos_core::report_diff::{
    diff_reports, format_diff_json, format_diff_markdown, format_diff_summary,
};
use serde_json::Value;

#[test]
fn diff_reports_only_tracks_finding_changes_and_ignores_report_metadata() {
    let before = report_v2(
        "/repo/before",
        Some("/repo/before/kratos.config.json"),
        Some("2026-04-22T00:00:00Z"),
        summary_counts(1, 2, 3, 4, 5, 6, 7, 8, 9),
        finding_set(
            vec![
                broken_import("/repo/src/a.ts", "./missing"),
                broken_import("/repo/src/shared.ts", "./shared"),
            ],
            vec![
                orphan_file("/repo/src/orphan-a.ts", OrphanKind::Module, "A", 0.91),
                orphan_file("/repo/src/orphan-b.ts", OrphanKind::Component, "B", 0.82),
            ],
            vec![
                dead_export("/repo/src/dead-a.ts", "old"),
                dead_export("/repo/src/dead-b.ts", "keep"),
            ],
            vec![
                unused_import("/repo/src/use-a.ts", "./lib", "old", "Old"),
                unused_import("/repo/src/use-a.ts", "./lib", "keep", "Keep"),
            ],
            vec![route_entrypoint(
                "/repo/src/routes.ts",
                EntrypointKind::NextAppRoute,
            )],
            vec![
                deletion_candidate("/repo/src/delete-a.ts", "unused", 0.98, true),
                deletion_candidate("/repo/src/delete-b.ts", "kept", 0.77, false),
            ],
        ),
        vec![module("/repo/src/one.ts")],
    );
    let after = report_v2(
        "/repo/after",
        None,
        Some("2026-04-22T01:00:00Z"),
        summary_counts(10, 20, 30, 40, 50, 60, 70, 80, 90),
        finding_set(
            vec![
                broken_import("/repo/src/shared.ts", "./shared"),
                broken_import("/repo/src/b.ts", "./introduced"),
            ],
            vec![
                orphan_file("/repo/src/orphan-b.ts", OrphanKind::Component, "B", 0.82),
                orphan_file("/repo/src/orphan-c.ts", OrphanKind::RouteModule, "C", 0.73),
            ],
            vec![dead_export("/repo/src/dead-b.ts", "keep")],
            vec![
                unused_import("/repo/src/use-a.ts", "./lib", "keep", "Keep"),
                unused_import("/repo/src/use-a.ts", "./lib", "new", "New"),
            ],
            vec![
                route_entrypoint("/repo/src/routes.ts", EntrypointKind::NextAppRoute),
                route_entrypoint("/repo/src/route-tool.ts", EntrypointKind::ToolingEntry),
            ],
            vec![
                deletion_candidate("/repo/src/delete-b.ts", "kept", 0.77, false),
                deletion_candidate("/repo/src/delete-c.ts", "new", 0.65, true),
            ],
        ),
        vec![module("/repo/src/two.ts"), module("/repo/src/three.ts")],
    );

    let diff = diff_reports(&before, &after);

    assert_eq!(diff.summary.broken_imports.introduced, 1);
    assert_eq!(diff.summary.broken_imports.resolved, 1);
    assert_eq!(diff.summary.broken_imports.persisted, 1);
    assert_eq!(diff.summary.orphan_files.introduced, 1);
    assert_eq!(diff.summary.orphan_files.resolved, 1);
    assert_eq!(diff.summary.orphan_files.persisted, 1);
    assert_eq!(diff.summary.dead_exports.introduced, 0);
    assert_eq!(diff.summary.dead_exports.resolved, 1);
    assert_eq!(diff.summary.dead_exports.persisted, 1);
    assert_eq!(diff.summary.unused_imports.introduced, 1);
    assert_eq!(diff.summary.unused_imports.resolved, 1);
    assert_eq!(diff.summary.unused_imports.persisted, 1);
    assert_eq!(diff.summary.route_entrypoints.introduced, 1);
    assert_eq!(diff.summary.route_entrypoints.resolved, 0);
    assert_eq!(diff.summary.route_entrypoints.persisted, 1);
    assert_eq!(diff.summary.deletion_candidates.introduced, 1);
    assert_eq!(diff.summary.deletion_candidates.resolved, 1);
    assert_eq!(diff.summary.deletion_candidates.persisted, 1);
    assert_eq!(diff.summary.totals.introduced, 5);
    assert_eq!(diff.summary.totals.resolved, 5);
    assert_eq!(diff.summary.totals.persisted, 6);

    assert_eq!(
        diff.findings.broken_imports.introduced,
        vec![broken_import("/repo/src/b.ts", "./introduced")]
    );
    assert_eq!(
        diff.findings.broken_imports.resolved,
        vec![broken_import("/repo/src/a.ts", "./missing")]
    );
    assert_eq!(
        diff.findings.broken_imports.persisted,
        vec![broken_import("/repo/src/shared.ts", "./shared")]
    );
    assert_eq!(
        diff.findings.route_entrypoints.introduced,
        vec![route_entrypoint(
            "/repo/src/route-tool.ts",
            EntrypointKind::ToolingEntry,
        )]
    );
    assert_eq!(
        diff.findings.route_entrypoints.persisted,
        vec![route_entrypoint(
            "/repo/src/routes.ts",
            EntrypointKind::NextAppRoute,
        )]
    );
    assert_eq!(
        diff.findings.deletion_candidates.resolved,
        vec![deletion_candidate(
            "/repo/src/delete-a.ts",
            "unused",
            0.98,
            true
        )]
    );
}

#[test]
fn diff_formatters_render_summary_markdown_and_json() {
    let before = report_v2(
        "/repo/before",
        None,
        None,
        SummaryCounts::default(),
        finding_set(
            vec![broken_import("/repo/src/a.ts", "./missing")],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        vec![],
    );
    let after = report_v2(
        "/repo/after",
        None,
        None,
        SummaryCounts::default(),
        finding_set(
            vec![broken_import("/repo/src/b.ts", "./introduced")],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        vec![],
    );
    let diff = diff_reports(&before, &after);
    let before_path = PathBuf::from("/tmp/before-report.json");
    let after_path = PathBuf::from("/tmp/after-report.json");

    let summary =
        format_diff_summary(&diff, &before_path, &after_path).expect("summary should format");
    assert!(summary.contains("Kratos diff 완료."));
    assert!(summary.contains("이전: /tmp/before-report.json"));
    assert!(summary.contains("깨진 import: 새로 발생 1, 해결됨 1, 유지됨 0"));
    assert!(summary.contains("합계: 새로 발생 1, 해결됨 1, 유지됨 0"));

    let markdown =
        format_diff_markdown(&diff, &before_path, &after_path).expect("markdown should format");
    assert!(markdown.contains("# Kratos Diff 결과"));
    assert!(markdown.contains("## 깨진 import"));
    assert!(markdown.contains("### 새로 발생 (1)"));
    assert!(markdown.contains("### 해결됨 (1)"));
    assert!(markdown.contains("### 유지됨 (0)"));
    assert!(markdown.contains("- 없음"));

    let json = format_diff_json(&diff, &before_path, &after_path).expect("json should format");
    let value: Value = serde_json::from_str(&json).expect("diff json should parse");
    assert_eq!(
        value["before"]["path"],
        Value::from("/tmp/before-report.json")
    );
    assert_eq!(
        value["after"]["path"],
        Value::from("/tmp/after-report.json")
    );
    assert_eq!(
        value["summary"]["brokenImports"]["introduced"],
        Value::from(1)
    );
    assert_eq!(
        value["findings"]["brokenImports"]["introduced"][0]["file"],
        Value::from("/repo/src/b.ts")
    );
}

#[test]
fn diff_formatters_keep_human_markdown_labels_in_korean() {
    let empty_report = report_v2(
        "/repo/empty",
        None,
        None,
        SummaryCounts::default(),
        finding_set(vec![], vec![], vec![], vec![], vec![], vec![]),
        vec![],
    );
    let empty_diff = diff_reports(&empty_report, &empty_report);
    let before_path = PathBuf::from("/tmp/before-report.json");
    let after_path = PathBuf::from("/tmp/after-report.json");
    let empty_summary = format_diff_summary(&empty_diff, &before_path, &after_path)
        .expect("empty summary should format");
    assert!(empty_summary.contains("변경된 항목이 없습니다."));

    let before = report_v2(
        "/repo/before",
        None,
        None,
        SummaryCounts::default(),
        finding_set(vec![], vec![], vec![], vec![], vec![], vec![]),
        vec![],
    );
    let after = report_v2(
        "/repo/after",
        None,
        None,
        SummaryCounts::default(),
        finding_set(
            vec![],
            vec![],
            vec![],
            vec![unused_import(
                "/repo/after/src/use.ts",
                "./lib",
                "helper",
                "helper",
            )],
            vec![route_entrypoint(
                "/repo/after/src/page.tsx",
                EntrypointKind::NextAppRoute,
            )],
            vec![deletion_candidate(
                "/repo/after/src/delete.ts",
                "Module has no inbound references and is not treated as an entrypoint.",
                0.66,
                true,
            )],
        ),
        vec![],
    );
    let diff = diff_reports(&before, &after);
    let summary =
        format_diff_summary(&diff, &before_path, &after_path).expect("summary should format");
    let markdown =
        format_diff_markdown(&diff, &before_path, &after_path).expect("markdown should format");

    assert!(summary.contains("라우트 진입점: 새로 발생 1, 해결됨 0, 유지됨 0"));
    assert!(markdown.contains("## 라우트 진입점"));
    assert!(markdown.contains("- /repo/after/src/use.ts -> `helper` (출처: `./lib`)"));
    assert!(markdown.contains(
        "- /repo/after/src/delete.ts (모듈에 참조가 없고 진입점으로 취급되지 않습니다., 신뢰도 0.66)"
    ));
}

#[test]
fn diff_reports_treats_matching_relative_paths_as_persisted_across_root_changes() {
    let before = report_v2(
        "/repo/before",
        None,
        None,
        SummaryCounts::default(),
        finding_set(
            vec![broken_import("/repo/before/src/index.ts", "./shared")],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        vec![],
    );
    let after = report_v2(
        "/repo/after",
        None,
        None,
        SummaryCounts::default(),
        finding_set(
            vec![broken_import("/repo/after/src/index.ts", "./shared")],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        vec![],
    );

    let diff = diff_reports(&before, &after);

    assert_eq!(diff.summary.broken_imports.introduced, 0);
    assert_eq!(diff.summary.broken_imports.resolved, 0);
    assert_eq!(diff.summary.broken_imports.persisted, 1);
}

#[test]
fn diff_reports_tracks_duplicate_finding_count_changes() {
    let before = report_v2(
        "/repo/before",
        None,
        None,
        SummaryCounts::default(),
        finding_set(
            vec![
                broken_import("/repo/before/src/index.ts", "./shared"),
                broken_import("/repo/before/src/index.ts", "./shared"),
            ],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        vec![],
    );
    let after = report_v2(
        "/repo/after",
        None,
        None,
        SummaryCounts::default(),
        finding_set(
            vec![broken_import("/repo/after/src/index.ts", "./shared")],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        vec![],
    );

    let diff = diff_reports(&before, &after);

    assert_eq!(diff.summary.broken_imports.introduced, 0);
    assert_eq!(diff.summary.broken_imports.resolved, 1);
    assert_eq!(diff.summary.broken_imports.persisted, 1);
    assert_eq!(
        diff.findings.broken_imports.resolved,
        vec![broken_import("/repo/before/src/index.ts", "./shared")]
    );
    assert_eq!(
        diff.findings.broken_imports.persisted,
        vec![broken_import("/repo/after/src/index.ts", "./shared")]
    );
}

#[test]
fn diff_reports_treat_deletion_candidate_metadata_changes_as_real_changes() {
    let before = report_v2(
        "/repo/before",
        None,
        None,
        SummaryCounts::default(),
        finding_set(
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![deletion_candidate(
                "/repo/before/src/delete.ts",
                "unused",
                0.55,
                false,
            )],
        ),
        vec![],
    );
    let after = report_v2(
        "/repo/after",
        None,
        None,
        SummaryCounts::default(),
        finding_set(
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![deletion_candidate(
                "/repo/after/src/delete.ts",
                "unused but safer now",
                0.91,
                true,
            )],
        ),
        vec![],
    );

    let diff = diff_reports(&before, &after);

    assert_eq!(diff.summary.deletion_candidates.introduced, 1);
    assert_eq!(diff.summary.deletion_candidates.resolved, 1);
    assert_eq!(diff.summary.deletion_candidates.persisted, 0);
    assert_eq!(
        diff.findings.deletion_candidates.resolved,
        vec![deletion_candidate(
            "/repo/before/src/delete.ts",
            "unused",
            0.55,
            false,
        )]
    );
    assert_eq!(
        diff.findings.deletion_candidates.introduced,
        vec![deletion_candidate(
            "/repo/after/src/delete.ts",
            "unused but safer now",
            0.91,
            true,
        )]
    );
}

fn report_v2(
    root: &str,
    config_path: Option<&str>,
    generated_at: Option<&str>,
    summary: SummaryCounts,
    findings: FindingSet,
    modules: Vec<ModuleRecord>,
) -> ReportV2 {
    ReportV2 {
        version: 2,
        generated_at: generated_at.map(str::to_string),
        root: PathBuf::from(root),
        config_path: config_path.map(PathBuf::from),
        summary,
        findings,
        modules,
    }
}

fn summary_counts(
    files_scanned: usize,
    entrypoints: usize,
    broken_imports: usize,
    orphan_files: usize,
    dead_exports: usize,
    unused_imports: usize,
    route_entrypoints: usize,
    deletion_candidates: usize,
    suppressed_findings: usize,
) -> SummaryCounts {
    SummaryCounts {
        files_scanned,
        entrypoints,
        broken_imports,
        orphan_files,
        dead_exports,
        unused_imports,
        route_entrypoints,
        deletion_candidates,
        suppressed_findings,
    }
}

fn finding_set(
    broken_imports: Vec<BrokenImportFinding>,
    orphan_files: Vec<OrphanFileFinding>,
    dead_exports: Vec<DeadExportFinding>,
    unused_imports: Vec<UnusedImportFinding>,
    route_entrypoints: Vec<RouteEntrypointFinding>,
    deletion_candidates: Vec<DeletionCandidateFinding>,
) -> FindingSet {
    FindingSet {
        broken_imports,
        orphan_files,
        dead_exports,
        unused_imports,
        route_entrypoints,
        deletion_candidates,
    }
}

fn module(file: &str) -> ModuleRecord {
    ModuleRecord {
        file_path: PathBuf::from(file),
        relative_path: file.trim_start_matches('/').to_string(),
        ..ModuleRecord::default()
    }
}

fn broken_import(file: &str, source: &str) -> BrokenImportFinding {
    BrokenImportFinding {
        file: PathBuf::from(file),
        source: source.to_string(),
        kind: ImportKind::Static,
    }
}

fn orphan_file(file: &str, kind: OrphanKind, reason: &str, confidence: f32) -> OrphanFileFinding {
    OrphanFileFinding {
        file: PathBuf::from(file),
        kind,
        reason: reason.to_string(),
        confidence,
    }
}

fn dead_export(file: &str, export_name: &str) -> DeadExportFinding {
    DeadExportFinding {
        file: PathBuf::from(file),
        export_name: export_name.to_string(),
    }
}

fn unused_import(file: &str, source: &str, local: &str, imported: &str) -> UnusedImportFinding {
    UnusedImportFinding {
        file: PathBuf::from(file),
        source: source.to_string(),
        local: local.to_string(),
        imported: imported.to_string(),
    }
}

fn route_entrypoint(file: &str, kind: EntrypointKind) -> RouteEntrypointFinding {
    RouteEntrypointFinding {
        file: PathBuf::from(file),
        kind,
    }
}

fn deletion_candidate(
    file: &str,
    reason: &str,
    confidence: f32,
    safe: bool,
) -> DeletionCandidateFinding {
    DeletionCandidateFinding {
        file: PathBuf::from(file),
        reason: reason.to_string(),
        confidence,
        safe,
    }
}
