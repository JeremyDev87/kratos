mod support;

use support::cli::{run_cli, run_cli_in_dir};
use support::fs::{copy_demo_app, repo_root};

#[test]
fn root_help_matches_expected_shape() {
    let output = run_cli(&[]);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Kratos\nDestroy dead code ruthlessly.\n\nUsage:\n  kratos scan [root] [--output path] [--json] [--fail-on kinds]\n  kratos report [report-path-or-root] [--format summary|json|md] [--fail-on kinds]\n  kratos clean [report-path-or-root] [--dry-run|--apply]\n\nCommands:\n  scan    Analyze a codebase and save the latest report.\n  report  Print a saved report in summary, json, or markdown form.\n  clean   Show deletion candidates or delete them with --apply.\n"
    );
}

#[test]
fn unknown_command_returns_help_and_exit_code_one() {
    let output = run_cli(&["nope"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Unknown command: nope\n\nKratos\nDestroy dead code ruthlessly.\n\nUsage:\n  kratos scan [root] [--output path] [--json] [--fail-on kinds]\n  kratos report [report-path-or-root] [--format summary|json|md] [--fail-on kinds]\n  kratos clean [report-path-or-root] [--dry-run|--apply]\n\nCommands:\n  scan    Analyze a codebase and save the latest report.\n  report  Print a saved report in summary, json, or markdown form.\n  clean   Show deletion candidates or delete them with --apply.\n"
    );
}

#[test]
fn scan_report_and_clean_work_for_demo_fixture() {
    let project_root = copy_demo_app("cli-smoke");
    let report_path = project_root.join(".kratos/latest-report.json");

    let scan = run_cli(&["scan", project_root.to_str().expect("path should be utf8")]);
    assert!(scan.status.success());
    let scan_stdout = String::from_utf8_lossy(&scan.stdout);
    assert!(scan_stdout.contains("Kratos scan complete."));
    assert!(scan_stdout.contains("Files scanned: 5"));
    assert!(scan_stdout.contains("Broken imports: 1"));
    assert!(scan_stdout.contains("Route entrypoints: 1"));
    assert!(scan_stdout.contains("Deletion candidates: 2"));
    assert!(report_path.exists());

    let report = run_cli(&[
        "report",
        report_path.to_str().expect("path should be utf8"),
        "--format",
        "md",
    ]);
    assert!(report.status.success());
    let report_stdout = String::from_utf8_lossy(&report.stdout);
    assert!(report_stdout.contains("# Kratos Report"));
    assert!(report_stdout.contains("- Route entrypoints: 1"));
    assert!(report_stdout.contains("## Broken imports"));
    assert!(report_stdout.contains("## Route entrypoints"));
    assert!(report_stdout.contains("DeadWidget"));

    let clean = run_cli(&["clean", report_path.to_str().expect("path should be utf8")]);
    assert!(clean.status.success());
    let clean_stdout = String::from_utf8_lossy(&clean.stdout);
    assert!(clean_stdout.contains("Kratos clean dry run."));
    assert!(clean_stdout.contains("Re-run with --apply to delete these files."));
}

#[test]
fn clean_accepts_legacy_v1_reports_through_cli() {
    let project_root = copy_demo_app("cli-clean-v1-report");
    let source_report = repo_root().join("fixtures/parity/demo-app/latest-report.v1.json");
    let report_path = project_root.join("latest-report.v1.json");
    let report_body = std::fs::read_to_string(&source_report)
        .expect("source report should read")
        .replace(
            "<ROOT>",
            project_root.to_str().expect("path should be utf8"),
        );
    std::fs::write(&report_path, report_body).expect("legacy report should write");

    let dry_run = run_cli_in_dir(
        &project_root,
        &["clean", report_path.to_str().expect("path should be utf8")],
    );
    assert!(dry_run.status.success());
    assert!(String::from_utf8_lossy(&dry_run.stdout).contains("Kratos clean dry run."));

    let apply = run_cli_in_dir(
        &project_root,
        &[
            "clean",
            "--apply",
            report_path.to_str().expect("path should be utf8"),
        ],
    );
    assert!(apply.status.success());
    assert!(String::from_utf8_lossy(&apply.stdout).contains("Kratos clean deleted 2 file(s)."));
    assert!(!project_root.join("src/components/DeadWidget.tsx").exists());
    assert!(!project_root.join("src/lib/broken.ts").exists());
}

#[test]
fn unknown_flags_and_surplus_positionals_match_js_baseline() {
    let project_root = copy_demo_app("cli-js-baseline");
    let report_path = project_root.join(".kratos/latest-report.json");

    let scan = run_cli_in_dir(&project_root, &["scan", "--bogus"]);
    assert!(scan.status.success());
    assert!(report_path.exists());
    assert!(
        !project_root.join("--bogus").exists(),
        "unknown flag should not redirect scan root"
    );

    let report = run_cli_in_dir(
        &project_root,
        &[
            "report",
            report_path.to_str().expect("path should be utf8"),
            "extra",
        ],
    );
    assert!(report.status.success());
    assert!(String::from_utf8_lossy(&report.stdout).contains("Kratos report."));

    let clean = run_cli_in_dir(&project_root, &["clean", "--bogus"]);
    assert!(clean.status.success());
    assert!(String::from_utf8_lossy(&clean.stdout).contains("Kratos clean dry run."));
}

#[test]
fn invalid_report_format_is_an_error_per_plan_contract() {
    let project_root = copy_demo_app("cli-invalid-format");
    let report_path = project_root.join(".kratos/latest-report.json");

    let scan = run_cli_in_dir(&project_root, &["scan"]);
    assert!(scan.status.success());
    assert!(report_path.exists());

    let report = run_cli_in_dir(
        &project_root,
        &[
            "report",
            report_path.to_str().expect("path should be utf8"),
            "--format",
            "bogus",
        ],
    );
    assert_eq!(report.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&report.stderr)
        .contains("Kratos failed: Config error: Invalid report format: bogus"));

    let hyphenated_report = run_cli_in_dir(
        &project_root,
        &[
            "report",
            report_path.to_str().expect("path should be utf8"),
            "--format",
            "-foo",
        ],
    );
    assert_eq!(hyphenated_report.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&hyphenated_report.stderr)
        .contains("Kratos failed: Config error: Invalid report format: -foo"));
}

#[test]
fn report_json_pretty_prints_the_parsed_input_shape() {
    let project_root = copy_demo_app("cli-json-report");
    let source_report = repo_root().join("fixtures/parity/demo-app/latest-report.v1.json");
    let minified_report = project_root.join("latest-report.v1.min.json");
    let source_value: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&source_report).expect("source report should read"),
    )
    .expect("source report should parse");
    std::fs::write(
        &minified_report,
        serde_json::to_string(&source_value).expect("minified report should serialize"),
    )
    .expect("minified report should write");

    let output = run_cli_in_dir(
        &project_root,
        &[
            "report",
            minified_report.to_str().expect("path should be utf8"),
            "--format",
            "json",
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("{\n"));
    assert!(stdout.contains("\n  \"schemaVersion\": 2,"));
    assert!(stdout.contains("\n  \"summary\": {"));
}

#[test]
fn report_json_accepts_arbitrary_json_and_empty_or_missing_format_falls_back_to_summary() {
    let project_root = copy_demo_app("cli-report-json-any");
    let arbitrary_json = project_root.join("arbitrary.json");
    std::fs::write(&arbitrary_json, "{\"hello\":\"world\",\"ok\":true}\n")
        .expect("arbitrary json should write");

    let json_output = run_cli_in_dir(
        &project_root,
        &[
            "report",
            arbitrary_json.to_str().expect("path should be utf8"),
            "--format",
            "json",
        ],
    );
    assert!(json_output.status.success());
    let json_stdout = String::from_utf8_lossy(&json_output.stdout);
    assert!(json_stdout.starts_with("{\n"));
    assert!(json_stdout.contains("\n  \"hello\": \"world\","));

    let report_path = project_root.join(".kratos/latest-report.json");
    let scan = run_cli_in_dir(&project_root, &["scan"]);
    assert!(scan.status.success());
    assert!(report_path.exists());

    let bare_format = run_cli_in_dir(
        &project_root,
        &[
            "report",
            report_path.to_str().expect("path should be utf8"),
            "--format",
        ],
    );
    assert!(bare_format.status.success());
    assert!(String::from_utf8_lossy(&bare_format.stdout).contains("Kratos report."));

    let empty_inline_format = run_cli_in_dir(
        &project_root,
        &[
            "report",
            report_path.to_str().expect("path should be utf8"),
            "--format=",
        ],
    );
    assert!(empty_inline_format.status.success());
    assert!(String::from_utf8_lossy(&empty_inline_format.stdout).contains("Kratos report."));
}

#[test]
fn report_markdown_uses_undefined_for_missing_generated_at() {
    let project_root = copy_demo_app("cli-report-md-missing-generated");
    let report_path = project_root.join("report-no-generated.json");
    std::fs::write(
        &report_path,
        "{\"schemaVersion\":2,\"project\":{\"root\":\"/tmp/demo\",\"configPath\":null},\"summary\":{\"filesScanned\":0,\"entrypoints\":0,\"brokenImports\":0,\"orphanFiles\":0,\"deadExports\":0,\"unusedImports\":0,\"routeEntrypoints\":0,\"deletionCandidates\":0},\"findings\":{\"brokenImports\":[],\"orphanFiles\":[],\"deadExports\":[],\"unusedImports\":[],\"routeEntrypoints\":[],\"deletionCandidates\":[]},\"graph\":{\"modules\":[]}}\n",
    )
    .expect("report should write");

    let output = run_cli_in_dir(
        &project_root,
        &[
            "report",
            report_path.to_str().expect("path should be utf8"),
            "--format",
            "md",
        ],
    );

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("- Generated: undefined"));
}

#[test]
fn report_summary_and_markdown_accept_future_schema_versions() {
    let project_root = copy_demo_app("cli-report-future-schema");
    let report_path = project_root.join("report-v3.json");
    std::fs::write(
        &report_path,
        "{\"schemaVersion\":3,\"project\":{\"root\":\"/tmp/demo\",\"configPath\":null},\"summary\":{\"filesScanned\":0,\"entrypoints\":0,\"brokenImports\":0,\"orphanFiles\":0,\"deadExports\":0,\"unusedImports\":0,\"routeEntrypoints\":0,\"deletionCandidates\":0},\"findings\":{\"brokenImports\":[],\"orphanFiles\":[],\"deadExports\":[],\"unusedImports\":[],\"routeEntrypoints\":[],\"deletionCandidates\":[]},\"graph\":{\"modules\":[]}}\n",
    )
    .expect("report should write");

    let summary = run_cli_in_dir(
        &project_root,
        &[
            "report",
            report_path.to_str().expect("path should be utf8"),
            "--format",
            "summary",
        ],
    );
    assert!(summary.status.success());
    assert!(String::from_utf8_lossy(&summary.stdout).contains("Kratos report."));

    let markdown = run_cli_in_dir(
        &project_root,
        &[
            "report",
            report_path.to_str().expect("path should be utf8"),
            "--format",
            "md",
        ],
    );
    assert!(markdown.status.success());
    assert!(String::from_utf8_lossy(&markdown.stdout).contains("# Kratos Report"));
}

#[test]
fn report_incomplete_future_schema_fails_fast() {
    let project_root = copy_demo_app("cli-report-incomplete-future-schema");
    let report_path = project_root.join("report-v3-min.json");
    std::fs::write(&report_path, "{\"schemaVersion\":3,\"project\":{\"root\":\"/tmp/demo\"}}\n")
        .expect("report should write");

    let summary = run_cli_in_dir(
        &project_root,
        &[
            "report",
            report_path.to_str().expect("path should be utf8"),
            "--format",
            "summary",
        ],
    );
    assert_eq!(summary.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&summary.stderr).contains("required object `summary`"));

    let markdown = run_cli_in_dir(
        &project_root,
        &[
            "report",
            report_path.to_str().expect("path should be utf8"),
            "--format",
            "md",
        ],
    );
    assert_eq!(markdown.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&markdown.stderr).contains("required object `summary`"));
}

#[test]
fn clean_accepts_future_schema_reports_when_the_shape_is_compatible() {
    let project_root = copy_demo_app("cli-clean-future-schema");
    let report_path = project_root.join("report-v3-clean.json");
    let dead_file = project_root.join("dead.txt");
    std::fs::write(&dead_file, "dead\n").expect("dead file should write");
    std::fs::write(
        &report_path,
        format!(
            "{{\"schemaVersion\":3,\"generatedAt\":\"2026-04-21T00:00:00Z\",\"project\":{{\"root\":\"{}\",\"configPath\":null}},\"summary\":{{\"filesScanned\":1,\"entrypoints\":0,\"brokenImports\":0,\"orphanFiles\":0,\"deadExports\":0,\"unusedImports\":0,\"routeEntrypoints\":0,\"deletionCandidates\":1}},\"findings\":{{\"brokenImports\":[],\"orphanFiles\":[],\"deadExports\":[],\"unusedImports\":[],\"routeEntrypoints\":[],\"deletionCandidates\":[{{\"file\":\"{}\",\"reason\":\"test\",\"confidence\":1.0,\"safe\":true}}]}},\"graph\":{{\"modules\":[]}}}}\n",
            project_root.display(),
            dead_file.display(),
        ),
    )
    .expect("report should write");

    let dry_run = run_cli_in_dir(
        &project_root,
        &[
            "clean",
            report_path.to_str().expect("path should be utf8"),
        ],
    );
    assert!(dry_run.status.success());
    let dry_run_stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(dry_run_stdout.contains("Kratos clean dry run."));
    assert!(dry_run_stdout.contains("dead.txt"));
    assert!(dead_file.exists());

    let apply = run_cli_in_dir(
        &project_root,
        &[
            "clean",
            "--apply",
            report_path.to_str().expect("path should be utf8"),
        ],
    );
    assert!(apply.status.success());
    assert!(String::from_utf8_lossy(&apply.stdout).contains("Kratos clean deleted 1 file(s)."));
    assert!(!dead_file.exists());
}

#[test]
fn scan_output_empty_string_defaults_and_missing_value_errors() {
    let project_root = copy_demo_app("cli-output-edge");
    let report_path = project_root.join(".kratos/latest-report.json");

    let empty_output = run_cli_in_dir(&project_root, &["scan", "--output="]);
    assert!(empty_output.status.success());
    assert!(report_path.exists());

    let missing_output = run_cli_in_dir(&project_root, &["scan", "--output", "--json"]);
    assert_eq!(missing_output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&missing_output.stderr)
        .contains("Kratos failed: Config error: --output requires a path value"));
    assert!(
        !project_root.join("--json").exists(),
        "missing output value should not create a stray report file"
    );
}

#[test]
fn scan_and_report_support_fail_on_ci_gate() {
    let project_root = copy_demo_app("cli-fail-on");
    let report_path = project_root.join(".kratos/latest-report.json");

    let scan = run_cli_in_dir(
        &project_root,
        &["scan", "--fail-on", "broken-imports,deletion-candidates"],
    );
    assert_eq!(scan.status.code(), Some(2));
    let scan_stdout = String::from_utf8_lossy(&scan.stdout);
    assert!(scan_stdout.contains("Kratos scan complete."));
    assert!(scan_stdout.contains("Gate status: failed"));
    assert!(scan_stdout.contains("broken imports: 1"));
    assert!(scan_stdout.contains("deletion candidates: 2"));
    assert!(report_path.exists());

    let report = run_cli_in_dir(
        &project_root,
        &[
            "report",
            report_path.to_str().expect("path should be utf8"),
            "--format",
            "md",
            "--fail-on",
            "any",
        ],
    );
    assert_eq!(report.status.code(), Some(2));
    let report_stdout = String::from_utf8_lossy(&report.stdout);
    assert!(report_stdout.contains("## Gate Status"));
    assert!(report_stdout.contains("- Result: failed"));
}

#[test]
fn clean_supports_explicit_dry_run_and_rejects_conflicting_flags() {
    let project_root = copy_demo_app("cli-clean-flags");
    let report_path = project_root.join(".kratos/latest-report.json");

    let prepare_report = run_cli_in_dir(&project_root, &["scan"]);
    assert!(prepare_report.status.success());
    assert!(report_path.exists());

    let dry_run = run_cli_in_dir(
        &project_root,
        &[
            "clean",
            "--dry-run",
            report_path.to_str().expect("path should be utf8"),
        ],
    );
    assert!(dry_run.status.success());
    assert!(String::from_utf8_lossy(&dry_run.stdout).contains("Kratos clean dry run."));

    let conflict = run_cli_in_dir(
        &project_root,
        &[
            "clean",
            "--dry-run",
            "--apply",
            report_path.to_str().expect("path should be utf8"),
        ],
    );
    assert_eq!(conflict.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&conflict.stderr)
        .contains("Kratos failed: Config error: --apply and --dry-run cannot be used together"));
}

#[test]
fn boolean_flags_do_not_consume_following_positionals() {
    let project_root = copy_demo_app("cli-boolean-positional");
    let report_path = project_root.join(".kratos/latest-report.json");

    let scan = run_cli(&[
        "scan",
        "--json",
        project_root.to_str().expect("path should be utf8"),
    ]);
    assert!(scan.status.success());
    assert!(String::from_utf8_lossy(&scan.stdout)
        .contains(&format!("\"root\": \"{}\"", project_root.display())));

    let prepare_report = run_cli_in_dir(&project_root, &["scan"]);
    assert!(prepare_report.status.success());
    assert!(report_path.exists());

    let clean = run_cli_in_dir(
        &project_root,
        &[
            "clean",
            "--apply",
            report_path.to_str().expect("path should be utf8"),
        ],
    );
    assert!(clean.status.success());
    assert!(String::from_utf8_lossy(&clean.stdout).contains("Kratos clean deleted 2 file(s)."));
    assert!(!project_root.join("src/components/DeadWidget.tsx").exists());
    assert!(!project_root.join("src/lib/broken.ts").exists());
}

#[test]
fn empty_inline_boolean_flags_stay_falsey_like_js() {
    let project_root = copy_demo_app("cli-inline-empty-bools");
    let report_path = project_root.join(".kratos/latest-report.json");

    let scan = run_cli_in_dir(
        &project_root,
        &[
            "scan",
            "--json=",
            project_root.to_str().expect("path should be utf8"),
        ],
    );
    assert!(scan.status.success());
    let scan_stdout = String::from_utf8_lossy(&scan.stdout);
    assert!(scan_stdout.contains("Kratos scan complete."));
    assert!(!scan_stdout.trim_start().starts_with('{'));

    let prepare_report = run_cli_in_dir(&project_root, &["scan"]);
    assert!(prepare_report.status.success());
    assert!(report_path.exists());

    let clean = run_cli_in_dir(
        &project_root,
        &[
            "clean",
            "--apply=",
            report_path.to_str().expect("path should be utf8"),
        ],
    );
    assert!(clean.status.success());
    let clean_stdout = String::from_utf8_lossy(&clean.stdout);
    assert!(clean_stdout.contains("Kratos clean dry run."));
    assert!(project_root.join("src/components/DeadWidget.tsx").exists());
    assert!(project_root.join("src/lib/broken.ts").exists());
}
