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
