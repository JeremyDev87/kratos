mod support;

use kratos_core::clean::{current_file_identity, current_parent_identity};
use support::cli::{run_cli, run_cli_in_dir};
use support::fs::{copy_demo_app, repo_root};

#[test]
fn root_help_matches_expected_shape() {
    let output = run_cli(&[]);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Kratos\n죽은 코드를 가차 없이 제거합니다.\n\n사용법:\n  kratos scan [root] [--output path] [--no-write] [--json]\n  kratos report [report-path-or-root] [--format summary|json|md]\n  kratos diff [before-report-path-or-root] [after-report-path-or-root] [--format summary|json|md]\n  kratos clean [report-path-or-root] [--apply] [--min-confidence value]\n\n명령:\n  scan    코드베이스를 분석하고 최신 리포트를 저장합니다.\n  report  저장된 리포트를 summary, json, markdown 형식으로 출력합니다.\n  diff    저장된 두 리포트를 비교합니다.\n  clean   삭제 후보를 표시하거나 --apply로 보존 격리합니다.\n"
    );
}

#[test]
fn root_version_flags_print_package_version() {
    for flag in ["--version", "-V"] {
        let output = run_cli(&[flag]);

        assert!(output.status.success(), "{flag} should exit successfully");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            format!("kratos {}\n", package_version())
        );
        assert!(output.stderr.is_empty(), "{flag} should not print stderr");
    }
}

fn package_version() -> String {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../package.json")).expect("valid package.json");
    manifest
        .get("version")
        .and_then(serde_json::Value::as_str)
        .expect("package.json version")
        .to_string()
}

#[test]
fn command_help_matches_korean_policy() {
    let output = run_cli(&["clean", "--help"]);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Kratos\n죽은 코드를 가차 없이 제거합니다.\n\nclean 명령\n삭제 후보를 표시하거나 --apply로 보존 격리합니다.\n\n사용법:\n  kratos clean [report-path-or-root] [--apply] [--min-confidence value]\n\n전체 명령을 보려면 `kratos --help`를 실행하세요.\n"
    );
}

#[test]
fn unknown_command_returns_help_and_exit_code_one() {
    let output = run_cli(&["nope"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "알 수 없는 명령: nope\n\nKratos\n죽은 코드를 가차 없이 제거합니다.\n\n사용법:\n  kratos scan [root] [--output path] [--no-write] [--json]\n  kratos report [report-path-or-root] [--format summary|json|md]\n  kratos diff [before-report-path-or-root] [after-report-path-or-root] [--format summary|json|md]\n  kratos clean [report-path-or-root] [--apply] [--min-confidence value]\n\n명령:\n  scan    코드베이스를 분석하고 최신 리포트를 저장합니다.\n  report  저장된 리포트를 summary, json, markdown 형식으로 출력합니다.\n  diff    저장된 두 리포트를 비교합니다.\n  clean   삭제 후보를 표시하거나 --apply로 보존 격리합니다.\n"
    );
}

#[test]
fn scan_report_and_clean_work_for_demo_fixture() {
    let project_root = copy_demo_app("cli-smoke");
    let report_path = project_root.join(".kratos/latest-report.json");

    let scan = run_cli(&["scan", project_root.to_str().expect("path should be utf8")]);
    assert!(scan.status.success());
    let scan_stdout = String::from_utf8_lossy(&scan.stdout);
    assert!(scan_stdout.contains("Kratos 스캔 완료."));
    assert!(scan_stdout.contains(
        "영향: 즉시 수정 대상 1개: 깨진 import 1개. 자동 정리 후보 2개: 삭제 후보 2개. 수동 검토 대상 3개: 사용되지 않는 export 3개."
    ));
    assert!(
        scan_stdout.contains("다음 권장 작업: 파일을 삭제하기 전에 깨진 import를 먼저 수정하세요.")
    );
    assert!(scan_stdout.contains("스캔한 파일: 5"));
    assert!(scan_stdout.contains("깨진 import: 1"));
    assert!(scan_stdout.contains("라우트 진입점: 1"));
    assert!(scan_stdout.contains("삭제 후보: 2"));
    assert!(scan_stdout.contains("다음 단계:"));
    assert!(scan_stdout.contains("쓰기 안내: 기본 리포트 경로 .kratos/latest-report.json는 체크아웃을 dirty하게 만들 수 있습니다. .gitignore에 .kratos/를 추가하거나 --output 또는 --no-write를 사용하세요."));
    assert!(scan_stdout.contains("npx로 실행 중이라면: npx @jeremyfellaz/kratos clean"));
    assert!(scan_stdout.contains("npx로 실행 중이라면: npx @jeremyfellaz/kratos report"));
    assert!(scan_stdout.contains("상위 정리 후보:"));
    assert!(scan_stdout.contains("- src/components/DeadWidget.tsx"));
    assert!(scan_stdout.contains(
        "설정 안내: kratos.config.json의 keepPatterns 또는 suppressions로 의도된 공개 API와 보존 파일을 고정하세요."
    ));
    assert!(!scan_stdout.contains("pages/home.tsx (next-pages-route)"));
    assert!(report_path.exists());

    let summary_report = run_cli(&[
        "report",
        report_path.to_str().expect("path should be utf8"),
        "--format",
        "summary",
    ]);
    assert!(summary_report.status.success());
    let summary_stdout = String::from_utf8_lossy(&summary_report.stdout);
    assert!(summary_stdout.contains(
        "영향: 즉시 수정 대상 1개: 깨진 import 1개. 자동 정리 후보 2개: 삭제 후보 2개. 수동 검토 대상 3개: 사용되지 않는 export 3개."
    ));
    assert!(summary_stdout.contains(
        "설정 안내: kratos.config.json의 keepPatterns 또는 suppressions로 의도된 공개 API와 보존 파일을 고정하세요."
    ));
    assert!(!summary_stdout.contains("pages/home.tsx (next-pages-route)"));

    let report = run_cli(&[
        "report",
        report_path.to_str().expect("path should be utf8"),
        "--format",
        "md",
    ]);
    assert!(report.status.success());
    let report_stdout = String::from_utf8_lossy(&report.stdout);
    assert!(report_stdout.contains("# Kratos 리포트"));
    assert!(report_stdout.contains("## 영향"));
    assert!(report_stdout.contains("- 라우트 진입점: 1"));
    assert!(report_stdout.contains("## 깨진 import"));
    assert!(report_stdout.contains("## 라우트 진입점"));
    assert!(report_stdout.contains("DeadWidget"));

    let clean = run_cli(&["clean", report_path.to_str().expect("path should be utf8")]);
    assert!(clean.status.success());
    let clean_stdout = String::from_utf8_lossy(&clean.stdout);
    assert!(clean_stdout.contains("Kratos clean 미리보기입니다."));
    assert!(clean_stdout.contains("상태: 존재함"));
    assert!(clean_stdout.contains("미리보기:"));
    assert!(clean_stdout.contains("export function DeadWidget()"));
    assert!(clean_stdout.contains("삭제하려면 --apply로 다시 실행하세요."));

    let diff = run_cli(&[
        "diff",
        report_path.to_str().expect("path should be utf8"),
        report_path.to_str().expect("path should be utf8"),
        "--format",
        "summary",
    ]);
    assert!(diff.status.success());
    let diff_stdout = String::from_utf8_lossy(&diff.stdout);
    assert!(diff_stdout.contains("Kratos diff 완료."));
    assert!(diff_stdout.contains("합계: 새로 발생 0, 해결됨 0, 유지됨 9"));

    let diff_json = run_cli(&[
        "diff",
        report_path.to_str().expect("path should be utf8"),
        report_path.to_str().expect("path should be utf8"),
        "--format",
        "json",
    ]);
    assert!(diff_json.status.success());
    let diff_json_value: serde_json::Value =
        serde_json::from_slice(&diff_json.stdout).expect("diff json should parse");
    assert_eq!(diff_json_value["summary"]["totals"]["persisted"], 9);
}

#[test]
fn scan_no_write_prints_human_summary_without_creating_default_report() {
    let project_root = copy_demo_app("cli-no-write");
    let report_path = project_root.join(".kratos/latest-report.json");

    let scan = run_cli_in_dir(&project_root, &["scan", "--no-write"]);

    assert!(scan.status.success());
    let scan_stdout = String::from_utf8_lossy(&scan.stdout);
    assert!(scan_stdout.contains("Kratos 스캔 완료."));
    assert!(scan_stdout.contains(
        "영향: 즉시 수정 대상 1개: 깨진 import 1개. 자동 정리 후보 2개: 삭제 후보 2개. 수동 검토 대상 3개: 사용되지 않는 export 3개."
    ));
    assert!(scan_stdout.contains("리포트 저장: --no-write 때문에 파일을 생성하지 않았습니다."));
    assert!(scan_stdout.contains(
        "리포트가 필요한 clean/report 작업은 기본 쓰기 모드로 다시 실행하거나 --output path를 지정하세요."
    ));
    assert!(!scan_stdout.contains("정리 미리보기: kratos clean"));
    assert!(!scan_stdout.contains("공유용 Markdown: kratos report"));
    assert!(!report_path.exists());
    assert!(!project_root.join(".kratos").exists());
}

#[test]
fn scan_no_write_rejects_output_flag() {
    let project_root = copy_demo_app("cli-no-write-output-conflict");

    let scan = run_cli_in_dir(
        &project_root,
        &["scan", "--no-write", "--output", "report.json"],
    );

    assert_eq!(scan.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&scan.stderr).contains(
        "Kratos 실행 실패: Config error: --output and --no-write cannot be used together"
    ));
    assert!(!project_root.join("report.json").exists());
    assert!(!project_root.join(".kratos").exists());

    let empty_output = run_cli_in_dir(&project_root, &["scan", "--no-write", "--output="]);
    assert_eq!(empty_output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&empty_output.stderr).contains(
        "Kratos 실행 실패: Config error: --output and --no-write cannot be used together"
    ));
}

#[test]
fn scan_json_stdout_stays_json_only() {
    let project_root = copy_demo_app("cli-json-only");

    let scan = run_cli_in_dir(&project_root, &["scan", "--json"]);

    assert!(scan.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&scan.stdout).expect("scan json should parse");
    assert_eq!(report["summary"]["filesScanned"], 5);
    let scan_stdout = String::from_utf8_lossy(&scan.stdout);
    assert!(!scan_stdout.contains("쓰기 안내"));
    assert!(!scan_stdout.contains("npx로 실행 중이라면"));
}

#[test]
fn scan_json_no_write_stays_json_only_without_creating_default_report() {
    let project_root = copy_demo_app("cli-json-no-write");
    let report_path = project_root.join(".kratos/latest-report.json");

    let scan = run_cli_in_dir(&project_root, &["scan", "--json", "--no-write"]);

    assert!(scan.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&scan.stdout).expect("scan json should parse");
    assert_eq!(report["summary"]["filesScanned"], 5);
    let scan_stdout = String::from_utf8_lossy(&scan.stdout);
    assert!(!scan_stdout.contains("쓰기 안내"));
    assert!(!scan_stdout.contains("npx로 실행 중이라면"));
    assert!(!report_path.exists());
    assert!(!project_root.join(".kratos").exists());
}

#[test]
fn scan_does_not_reopen_node_modules_for_broad_negated_ignore_patterns() {
    let project_root = copy_demo_app("cli-node-modules-ignore");
    std::fs::write(
        project_root.join("kratos.config.json"),
        r#"{
  "ignorePatterns": ["!**/*.ts"]
}
"#,
    )
    .expect("config should write");
    let dependency_dir = project_root.join("node_modules/@demo");
    std::fs::create_dir_all(&dependency_dir).expect("node_modules fixture should write");
    std::fs::write(
        dependency_dir.join("index.ts"),
        "export const dependency = true;\n",
    )
    .expect("dependency fixture should write");

    let scan = run_cli_in_dir(&project_root, &["scan", "--json"]);

    assert!(scan.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&scan.stdout).expect("scan json should parse");
    assert_eq!(report["summary"]["filesScanned"], 5);
    let modules = report["graph"]["modules"]
        .as_array()
        .expect("modules should be an array");
    assert!(
        modules.iter().all(|module| !module["relativePath"]
            .as_str()
            .unwrap_or_default()
            .starts_with("node_modules/")),
        "scan report should not include node_modules modules: {modules:#?}"
    );
}

#[test]
fn scan_respects_gitignore_and_config_overrides() {
    let project_root = copy_demo_app("cli-gitignore");
    std::fs::write(project_root.join(".gitignore"), "src/lib/**\n!src/lib/\n")
        .expect("gitignore should write");
    std::fs::write(
        project_root.join("kratos.config.json"),
        r#"{
  "ignorePatterns": ["!src/lib/math.ts"]
}
"#,
    )
    .expect("config should write");

    let scan = run_cli_in_dir(&project_root, &["scan", "--json"]);

    assert!(scan.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&scan.stdout).expect("scan json should parse");
    assert_eq!(report["summary"]["filesScanned"], 4);
    let relative_paths = report["graph"]["modules"]
        .as_array()
        .expect("modules should be an array")
        .iter()
        .filter_map(|module| module["relativePath"].as_str())
        .collect::<Vec<_>>();
    assert!(relative_paths.contains(&"src/lib/math.ts"));
    assert!(!relative_paths.contains(&"src/lib/broken.ts"));
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
    let dry_run_stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(dry_run_stdout.contains("Kratos clean 미리보기입니다."));
    assert!(dry_run_stdout.contains("삭제 대상: 0"));
    assert!(dry_run_stdout.contains("안전 검증으로 건너뛴 대상: 2"));

    let apply = run_cli_in_dir(
        &project_root,
        &[
            "clean",
            "--apply",
            report_path.to_str().expect("path should be utf8"),
        ],
    );
    assert!(apply.status.success());
    assert!(String::from_utf8_lossy(&apply.stdout)
        .contains("Kratos clean: 파일 0개를 코드 경로에서 격리했습니다."));
    assert!(project_root.join("src/components/DeadWidget.tsx").exists());
    assert!(project_root.join("src/lib/broken.ts").exists());
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
    assert!(String::from_utf8_lossy(&report.stdout).contains("Kratos 리포트."));

    let clean = run_cli_in_dir(&project_root, &["clean", "--bogus"]);
    assert!(clean.status.success());
    assert!(String::from_utf8_lossy(&clean.stdout).contains("Kratos clean 미리보기입니다."));
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
        .contains("Kratos 실행 실패: Config error: Invalid report format: bogus"));

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
        .contains("Kratos 실행 실패: Config error: Invalid report format: -foo"));
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
    assert!(String::from_utf8_lossy(&bare_format.stdout).contains("Kratos 리포트."));

    let empty_inline_format = run_cli_in_dir(
        &project_root,
        &[
            "report",
            report_path.to_str().expect("path should be utf8"),
            "--format=",
        ],
    );
    assert!(empty_inline_format.status.success());
    assert!(String::from_utf8_lossy(&empty_inline_format.stdout).contains("Kratos 리포트."));
}

#[test]
fn report_markdown_uses_korean_fallback_for_missing_generated_at() {
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("- 생성 시각: 정의되지 않음"));
}

#[test]
fn report_summary_and_markdown_accept_future_schema_versions() {
    let project_root = copy_demo_app("cli-report-future-schema");
    let report_path = project_root.join("report-v3.json");
    std::fs::write(
        &report_path,
        "{\"schemaVersion\":4,\"project\":{\"root\":\"/tmp/demo\",\"configPath\":null},\"summary\":{\"filesScanned\":0,\"entrypoints\":0,\"brokenImports\":0,\"orphanFiles\":0,\"deadExports\":0,\"unusedImports\":0,\"routeEntrypoints\":0,\"deletionCandidates\":0},\"findings\":{\"brokenImports\":[],\"orphanFiles\":[],\"deadExports\":[],\"unusedImports\":[],\"routeEntrypoints\":[],\"deletionCandidates\":[]},\"cleanSafety\":{\"fingerprintAlgorithm\":\"sha256\",\"candidates\":[]},\"graph\":{\"modules\":[]}}\n",
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
    assert!(String::from_utf8_lossy(&summary.stdout).contains("Kratos 리포트."));

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
    assert!(String::from_utf8_lossy(&markdown.stdout).contains("# Kratos 리포트"));
}

#[test]
fn report_incomplete_future_schema_fails_fast() {
    let project_root = copy_demo_app("cli-report-incomplete-future-schema");
    let report_path = project_root.join("report-v3-min.json");
    std::fs::write(
        &report_path,
        "{\"schemaVersion\":4,\"project\":{\"root\":\"/tmp/demo\"},\"cleanSafety\":{\"fingerprintAlgorithm\":\"sha256\",\"candidates\":[]}}\n",
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
    let identity = current_file_identity(&dead_file);
    let parent_identity = current_parent_identity(&dead_file);
    let safe = identity.is_some() && parent_identity.is_some();
    let identity_json = identity
        .as_deref()
        .map(|value| format!("\"{value}\""))
        .unwrap_or_else(|| "null".to_string());
    let parent_identity_json = parent_identity
        .as_deref()
        .map(|value| format!("\"{value}\""))
        .unwrap_or_else(|| "null".to_string());
    std::fs::write(
        &report_path,
        format!(
            "{{\"schemaVersion\":4,\"generatedAt\":\"2026-04-21T00:00:00Z\",\"project\":{{\"root\":\"{}\",\"configPath\":null}},\"summary\":{{\"filesScanned\":1,\"entrypoints\":0,\"brokenImports\":0,\"orphanFiles\":0,\"deadExports\":0,\"unusedImports\":0,\"routeEntrypoints\":0,\"deletionCandidates\":1}},\"findings\":{{\"brokenImports\":[],\"orphanFiles\":[],\"deadExports\":[],\"unusedImports\":[],\"routeEntrypoints\":[],\"deletionCandidates\":[{{\"file\":\"{}\",\"reason\":\"test\",\"confidence\":1.0,\"safe\":{safe}}}]}},\"cleanSafety\":{{\"fingerprintAlgorithm\":\"sha256\",\"candidates\":[{{\"file\":\"{}\",\"fingerprint\":\"9edc05076fb5a5921c7e8ffe2cc79cc5d711d9612e138d09572f76df4530d870\",\"identity\":{identity_json},\"parentIdentity\":{parent_identity_json}}}]}},\"graph\":{{\"modules\":[]}}}}\n",
            project_root.display(),
            dead_file.display(),
            dead_file.display(),
        ),
    )
    .expect("report should write");

    let dry_run = run_cli_in_dir(
        &project_root,
        &["clean", report_path.to_str().expect("path should be utf8")],
    );
    assert!(dry_run.status.success());
    let dry_run_stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(dry_run_stdout.contains("Kratos clean 미리보기입니다."));
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
    #[cfg(unix)]
    {
        assert!(String::from_utf8_lossy(&apply.stdout)
            .contains("Kratos clean: 파일 1개를 코드 경로에서 격리했습니다."));
        assert!(!dead_file.exists());
    }
    #[cfg(not(unix))]
    {
        let apply_stdout = String::from_utf8_lossy(&apply.stdout);
        assert!(apply_stdout.contains("Kratos clean: 파일 0개를 코드 경로에서 격리했습니다."));
        assert!(apply_stdout.contains("건너뛴 파일: 1"));
        assert!(dead_file.exists());
    }
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
        .contains("Kratos 실행 실패: Config error: --output requires a path value"));
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
    assert!(String::from_utf8_lossy(&clean.stdout)
        .contains("Kratos clean: 파일 2개를 코드 경로에서 격리했습니다."));
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
    assert!(scan_stdout.contains("Kratos 스캔 완료."));
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
    assert!(clean_stdout.contains("Kratos clean 미리보기입니다."));
    assert!(project_root.join("src/components/DeadWidget.tsx").exists());
    assert!(project_root.join("src/lib/broken.ts").exists());
}
