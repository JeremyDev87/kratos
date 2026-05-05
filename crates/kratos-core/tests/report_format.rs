use std::path::{Path, PathBuf};

use kratos_core::analyze::analyze_project;
use kratos_core::report::parse_report_json;
use kratos_core::report_format::{format_markdown_report, format_summary_report};

#[test]
fn summary_formatter_allows_custom_titles_without_changing_body_shape() {
    let repo_root = repo_root();
    let demo_root = repo_root.join("fixtures/demo-app");
    let report_path = demo_root.join(".kratos/latest-report.json");
    let report = analyze_project(&demo_root).expect("demo app should analyze");

    let rendered = format_summary_report(&report, &report_path, "Kratos scan complete.")
        .expect("summary should format");

    assert!(rendered.starts_with("Kratos 스캔 완료.\n\n"));
    assert!(rendered.contains(&format!("저장된 리포트: {}", report_path.display())));
    assert!(rendered.contains("깨진 import:"));
}

#[test]
fn markdown_formatter_uses_korean_fallback_when_generated_at_is_missing() {
    let report = parse_report_json(
        "{\"schemaVersion\":2,\"project\":{\"root\":\"/tmp/demo\",\"configPath\":null},\"summary\":{\"filesScanned\":0,\"entrypoints\":0,\"brokenImports\":0,\"orphanFiles\":0,\"deadExports\":0,\"unusedImports\":0,\"routeEntrypoints\":0,\"deletionCandidates\":0},\"findings\":{\"brokenImports\":[],\"orphanFiles\":[],\"deadExports\":[],\"unusedImports\":[],\"routeEntrypoints\":[],\"deletionCandidates\":[]},\"graph\":{\"modules\":[]}}",
    )
    .expect("report should parse");

    let rendered = format_markdown_report(&report, Path::new("/tmp/demo/report.json"))
        .expect("markdown should format");

    assert!(rendered.contains("- 생성 시각: 정의되지 않음"));
}

#[test]
fn summary_separates_cleanup_candidates_from_manual_review_dead_exports() {
    let report = parse_report_json(
        r#"{
          "schemaVersion": 2,
          "project": { "root": "/tmp/demo", "configPath": null },
          "summary": {
            "filesScanned": 2,
            "entrypoints": 0,
            "brokenImports": 0,
            "orphanFiles": 1,
            "deadExports": 2,
            "unusedImports": 0,
            "routeEntrypoints": 0,
            "deletionCandidates": 1
          },
          "findings": {
            "brokenImports": [],
            "orphanFiles": [
              {
                "file": "/tmp/demo/src/dead.ts",
                "kind": "orphan-module",
                "reason": "Module has no inbound references and is not treated as an entrypoint.",
                "confidence": 0.95
              }
            ],
            "deadExports": [
              { "file": "/tmp/demo/src/api.ts", "exportName": "unusedPublicApi" },
              { "file": "/tmp/demo/src/dead.ts", "exportName": "deadHelper" }
            ],
            "unusedImports": [],
            "routeEntrypoints": [],
            "deletionCandidates": [
              {
                "file": "/tmp/demo/src/dead.ts",
                "reason": "Module has no inbound references and is not treated as an entrypoint.",
                "confidence": 0.95,
                "safe": true
              }
            ]
          },
          "graph": { "modules": [] }
        }"#,
    )
    .expect("report should parse");

    let rendered = format_summary_report(
        &report,
        Path::new("/tmp/demo/.kratos/latest-report.json"),
        "Kratos report.",
    )
    .expect("summary should format");

    assert!(rendered.contains(
        "자동 정리 후보 1개: 삭제 후보 1개. 수동 검토 대상 2개: 사용되지 않는 export 2개."
    ));
    assert!(!rendered.contains("조치할 항목 3개"));
    assert!(!rendered.contains("자동 정리 후보 3개"));
    assert!(rendered.contains(
        "설정 안내: kratos.config.json의 keepPatterns 또는 suppressions로 의도된 공개 API와 보존 파일을 고정하세요."
    ));
}

#[test]
fn dead_export_only_summary_recommends_manual_review_instead_of_cleanup() {
    let report = parse_report_json(
        r#"{
          "schemaVersion": 2,
          "project": { "root": "/tmp/demo", "configPath": null },
          "summary": {
            "filesScanned": 1,
            "entrypoints": 0,
            "brokenImports": 0,
            "orphanFiles": 0,
            "deadExports": 1,
            "unusedImports": 0,
            "routeEntrypoints": 0,
            "deletionCandidates": 0
          },
          "findings": {
            "brokenImports": [],
            "orphanFiles": [],
            "deadExports": [
              { "file": "/tmp/demo/src/api.ts", "exportName": "unusedPublicApi" }
            ],
            "unusedImports": [],
            "routeEntrypoints": [],
            "deletionCandidates": []
          },
          "graph": { "modules": [] }
        }"#,
    )
    .expect("report should parse");

    let rendered = format_summary_report(
        &report,
        Path::new("/tmp/demo/.kratos/latest-report.json"),
        "Kratos report.",
    )
    .expect("summary should format");

    assert!(rendered.contains("수동 검토 대상 1개: 사용되지 않는 export 1개."));
    assert!(rendered.contains(
        "다음 권장 작업: 사용되지 않는 export는 삭제 후보가 아니므로 공개 API 여부를 수동 검토하세요."
    ));
    assert!(!rendered.contains("정리 미리보기:"));
}

#[test]
fn summary_does_not_repeat_orphan_files_already_shown_as_deletion_candidates() {
    let report = parse_report_json(
        r#"{
          "schemaVersion": 2,
          "project": { "root": "/tmp/demo", "configPath": null },
          "summary": {
            "filesScanned": 1,
            "entrypoints": 0,
            "brokenImports": 0,
            "orphanFiles": 1,
            "deadExports": 0,
            "unusedImports": 0,
            "routeEntrypoints": 0,
            "deletionCandidates": 1
          },
          "findings": {
            "brokenImports": [],
            "orphanFiles": [
              {
                "file": "/tmp/demo/src/dead.ts",
                "kind": "orphan-module",
                "reason": "Module has no inbound references and is not treated as an entrypoint.",
                "confidence": 0.95
              }
            ],
            "deadExports": [],
            "unusedImports": [],
            "routeEntrypoints": [],
            "deletionCandidates": [
              {
                "file": "/tmp/demo/src/dead.ts",
                "reason": "Module has no inbound references and is not treated as an entrypoint.",
                "confidence": 0.95,
                "safe": true
              }
            ]
          },
          "graph": { "modules": [] }
        }"#,
    )
    .expect("report should parse");

    let rendered = format_summary_report(
        &report,
        Path::new("/tmp/demo/.kratos/latest-report.json"),
        "Kratos report.",
    )
    .expect("summary should format");

    assert_eq!(rendered.matches("src/dead.ts").count(), 1);
    assert!(rendered.contains("정리 후보와 동일한 파일 1개는 위 목록에서 확인하세요."));
}

#[test]
fn summary_hides_route_entrypoint_details_but_markdown_keeps_them() {
    let report = parse_report_json(
        r#"{
          "schemaVersion": 2,
          "project": { "root": "/tmp/demo", "configPath": null },
          "summary": {
            "filesScanned": 1,
            "entrypoints": 1,
            "brokenImports": 0,
            "orphanFiles": 0,
            "deadExports": 0,
            "unusedImports": 0,
            "routeEntrypoints": 1,
            "deletionCandidates": 0
          },
          "findings": {
            "brokenImports": [],
            "orphanFiles": [],
            "deadExports": [],
            "unusedImports": [],
            "routeEntrypoints": [
              { "file": "/tmp/demo/pages/home.tsx", "kind": "next-pages-route" }
            ],
            "deletionCandidates": []
          },
          "graph": { "modules": [] }
        }"#,
    )
    .expect("report should parse");
    let report_path = Path::new("/tmp/demo/.kratos/latest-report.json");

    let summary = format_summary_report(&report, report_path, "Kratos report.")
        .expect("summary should format");
    let markdown = format_markdown_report(&report, report_path).expect("markdown should format");

    assert!(summary.contains("라우트 진입점: 1"));
    assert!(!summary.contains("pages/home.tsx (next-pages-route)"));
    assert!(markdown.contains("## 라우트 진입점"));
    assert!(markdown.contains("pages/home.tsx (next-pages-route)"));
}

#[test]
fn summary_and_markdown_formatters_accept_future_schema_versions() {
    let report = parse_report_json(
        "{\"schemaVersion\":3,\"project\":{\"root\":\"/tmp/demo\",\"configPath\":null},\"summary\":{\"filesScanned\":0,\"entrypoints\":0,\"brokenImports\":0,\"orphanFiles\":0,\"deadExports\":0,\"unusedImports\":0,\"routeEntrypoints\":0,\"deletionCandidates\":0},\"findings\":{\"brokenImports\":[],\"orphanFiles\":[],\"deadExports\":[],\"unusedImports\":[],\"routeEntrypoints\":[],\"deletionCandidates\":[]},\"graph\":{\"modules\":[]}}",
    )
    .expect("future-schema report should parse");

    let summary = format_summary_report(
        &report,
        Path::new("/tmp/demo/report.json"),
        "Kratos report.",
    )
    .expect("summary should format");
    let markdown = format_markdown_report(&report, Path::new("/tmp/demo/report.json"))
        .expect("markdown should format");

    assert!(summary.contains("Kratos 리포트."));
    assert!(summary.contains("저장된 리포트: /tmp/demo/report.json"));
    assert!(markdown.contains("# Kratos 리포트"));
    assert!(markdown.contains("- 리포트: /tmp/demo/report.json"));
}

#[test]
fn incomplete_future_schema_reports_fail_fast_instead_of_rendering_defaults() {
    let error = parse_report_json("{\"schemaVersion\":3,\"project\":{\"root\":\"/tmp/demo\"}}")
        .expect_err("incomplete future-schema report should fail");

    assert!(error.to_string().contains("required object `summary`"));
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root should exist")
        .to_path_buf()
}
