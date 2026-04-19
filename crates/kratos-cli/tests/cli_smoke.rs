use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn root_help_matches_expected_shape() {
    let output = run_cli(&[]);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Kratos\nDestroy dead code ruthlessly.\n\nUsage:\n  kratos scan [root] [--output path] [--json]\n  kratos report [report-path-or-root] [--format summary|json|md]\n  kratos clean [report-path-or-root] [--apply]\n\nCommands:\n  scan    Analyze a codebase and save the latest report.\n  report  Print a saved report in summary, json, or markdown form.\n  clean   Show deletion candidates or delete them with --apply.\n"
    );
}

#[test]
fn unknown_command_returns_help_and_exit_code_one() {
    let output = run_cli(&["nope"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Unknown command: nope\n\nKratos\nDestroy dead code ruthlessly.\n\nUsage:\n  kratos scan [root] [--output path] [--json]\n  kratos report [report-path-or-root] [--format summary|json|md]\n  kratos clean [report-path-or-root] [--apply]\n\nCommands:\n  scan    Analyze a codebase and save the latest report.\n  report  Print a saved report in summary, json, or markdown form.\n  clean   Show deletion candidates or delete them with --apply.\n"
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

fn run_cli(args: &[&str]) -> std::process::Output {
    run_cli_in_dir(&repo_root(), args)
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
    assert!(stdout.contains("\n  \"version\": 1,"));
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

fn run_cli_in_dir(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(cli_binary())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("cli command should run")
}

fn cli_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_kratos-cli"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root should exist")
        .to_path_buf()
}

fn copy_demo_app(label: &str) -> PathBuf {
    let destination = temp_dir(label).join("demo-app");
    copy_directory(&repo_root().join("fixtures/demo-app"), &destination);
    destination
}

fn copy_directory(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).expect("destination directory should exist");

    for entry in std::fs::read_dir(source).expect("source directory should be readable") {
        let entry = entry.expect("directory entry should load");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().expect("file type should load");

        if file_type.is_dir() {
            copy_directory(&source_path, &destination_path);
        } else if file_type.is_file() {
            std::fs::copy(&source_path, &destination_path).expect("file should copy");
        }
    }
}

fn temp_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kratos-cli-{label}-{unique}"));
    std::fs::create_dir_all(&path).expect("temp dir should be created");
    path
}
