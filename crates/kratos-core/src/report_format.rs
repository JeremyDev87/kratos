use std::path::Path;

use crate::model::ReportV2;
use crate::report::{entrypoint_kind_to_string, path_to_string};
use crate::KratosResult;

pub fn format_summary_report(
    report: &ReportV2,
    report_path: &Path,
    title: &str,
) -> KratosResult<String> {
    let impact = impact_summary(report);
    let mut lines = vec![
        display_title(title).to_string(),
        String::new(),
        format!("영향: {}", impact.headline),
        format!("다음 권장 작업: {}", impact.best_next_move),
        String::new(),
        format!("루트: {}", path_to_string(&report.root)),
        format!("스캔한 파일: {}", report.summary.files_scanned),
        format!("진입점: {}", report.summary.entrypoints),
        format!("깨진 import: {}", report.summary.broken_imports),
        format!("고아 파일: {}", report.summary.orphan_files),
        format!("사용되지 않는 export: {}", report.summary.dead_exports),
        format!("사용되지 않는 import: {}", report.summary.unused_imports),
        format!("라우트 진입점: {}", report.summary.route_entrypoints),
        format!("삭제 후보: {}", report.summary.deletion_candidates),
    ];
    if report.summary.suppressed_findings > 0 {
        lines.push(format!(
            "숨김 처리된 항목: {}",
            report.summary.suppressed_findings
        ));
    }
    lines.push(String::new());
    lines.push(format!("저장된 리포트: {}", path_to_string(report_path)));
    append_next_steps(&mut lines, report, report_path);

    append_preview(
        &mut lines,
        "깨진 import",
        &report.findings.broken_imports,
        |item| format!("{} -> {}", display_path(report, &item.file), item.source),
    );
    append_deletion_preview(&mut lines, report);
    append_preview(
        &mut lines,
        "고아 파일",
        &report.findings.orphan_files,
        |item| display_path(report, &item.file),
    );
    append_preview(
        &mut lines,
        "사용되지 않는 export",
        &report.findings.dead_exports,
        |item| format!("{}#{}", display_path(report, &item.file), item.export_name),
    );
    append_preview(
        &mut lines,
        "사용되지 않는 import",
        &report.findings.unused_imports,
        |item| {
            format!(
                "{} -> {} (출처: {})",
                display_path(report, &item.file),
                item.local,
                item.source
            )
        },
    );
    append_preview(
        &mut lines,
        "라우트 진입점",
        &report.findings.route_entrypoints,
        |item| {
            format!(
                "{} ({})",
                display_path(report, &item.file),
                entrypoint_kind_to_string(&item.kind)
            )
        },
    );

    Ok(lines.join("\n"))
}

pub fn format_markdown_report(report: &ReportV2, report_path: &Path) -> KratosResult<String> {
    let mut lines = vec![
        "# Kratos 리포트".to_string(),
        String::new(),
        format!("> {}", impact_summary(report).headline),
        String::new(),
        format!(
            "- 생성 시각: {}",
            report.generated_at.as_deref().unwrap_or("정의되지 않음")
        ),
        format!("- 루트: {}", path_to_string(&report.root)),
        format!("- 리포트: {}", path_to_string(report_path)),
        String::new(),
        "## 요약".to_string(),
        String::new(),
        format!("- 스캔한 파일: {}", report.summary.files_scanned),
        format!("- 진입점: {}", report.summary.entrypoints),
        format!("- 깨진 import: {}", report.summary.broken_imports),
        format!("- 고아 파일: {}", report.summary.orphan_files),
        format!("- 사용되지 않는 export: {}", report.summary.dead_exports),
        format!("- 사용되지 않는 import: {}", report.summary.unused_imports),
        format!("- 라우트 진입점: {}", report.summary.route_entrypoints),
        format!("- 삭제 후보: {}", report.summary.deletion_candidates),
    ];
    if report.summary.suppressed_findings > 0 {
        lines.push(format!(
            "- 숨김 처리된 항목: {}",
            report.summary.suppressed_findings
        ));
    }
    lines.push(String::new());

    push_markdown_impact(&mut lines, report, report_path);
    push_markdown_section(
        &mut lines,
        "깨진 import",
        &report.findings.broken_imports,
        |item| format!("{} -> `{}`", display_path(report, &item.file), item.source),
    );
    push_markdown_section(
        &mut lines,
        "고아 파일",
        &report.findings.orphan_files,
        |item| {
            format!(
                "{} ({})",
                display_path(report, &item.file),
                display_known_reason(&item.reason)
            )
        },
    );
    push_markdown_section(
        &mut lines,
        "사용되지 않는 export",
        &report.findings.dead_exports,
        |item| {
            format!(
                "{} -> `{}`",
                display_path(report, &item.file),
                item.export_name
            )
        },
    );
    push_markdown_section(
        &mut lines,
        "사용되지 않는 import",
        &report.findings.unused_imports,
        |item| {
            format!(
                "{} -> `{}` (출처: `{}`)",
                display_path(report, &item.file),
                item.local,
                item.source
            )
        },
    );
    push_markdown_section(
        &mut lines,
        "라우트 진입점",
        &report.findings.route_entrypoints,
        |item| {
            format!(
                "{} ({})",
                display_path(report, &item.file),
                entrypoint_kind_to_string(&item.kind)
            )
        },
    );
    push_markdown_section(
        &mut lines,
        "삭제 후보",
        &report.findings.deletion_candidates,
        |item| {
            format!(
                "{} ({}, 신뢰도 {})",
                display_path(report, &item.file),
                display_known_reason(&item.reason),
                item.confidence
            )
        },
    );

    Ok(lines.join("\n"))
}

struct ImpactSummary {
    headline: String,
    best_next_move: String,
}

fn impact_summary(report: &ReportV2) -> ImpactSummary {
    let breakages = report.summary.broken_imports;
    let cleanup_candidates = report.summary.deletion_candidates;
    let dead_exports = report.summary.dead_exports;
    let unused_imports = report.summary.unused_imports;
    let actionable_total = breakages + cleanup_candidates + dead_exports + unused_imports;

    let headline = if actionable_total == 0 {
        format!(
            "스캔한 파일 {}개에서 조치할 항목이 없습니다.",
            report.summary.files_scanned
        )
    } else {
        let mut parts = Vec::new();
        if breakages > 0 {
            parts.push(count_label(breakages, "깨진 import"));
        }
        if cleanup_candidates > 0 {
            parts.push(count_label(cleanup_candidates, "정리 후보"));
        }
        if dead_exports > 0 {
            parts.push(count_label(dead_exports, "사용되지 않는 export"));
        }
        if unused_imports > 0 {
            parts.push(count_label(unused_imports, "사용되지 않는 import"));
        }

        format!("조치할 항목 {}개: {}.", actionable_total, parts.join(", "))
    };

    let best_next_move = if breakages > 0 {
        "파일을 삭제하기 전에 깨진 import를 먼저 수정하세요.".to_string()
    } else if cleanup_candidates > 0 {
        "정리 미리보기를 실행하고 신뢰도 높은 미사용 파일을 제거하세요.".to_string()
    } else if dead_exports > 0 {
        "사용되지 않는 export를 제거하거나 의도된 공개 API인지 확인하세요.".to_string()
    } else if unused_imports > 0 {
        "사용되지 않는 import를 제거해 불필요한 의존성을 줄이세요.".to_string()
    } else {
        "향후 diff 기준선으로 JSON 리포트를 보관하세요.".to_string()
    };

    ImpactSummary {
        headline,
        best_next_move,
    }
}

fn count_label(count: usize, label: &str) -> String {
    format!("{label} {count}개")
}

fn append_next_steps(lines: &mut Vec<String>, report: &ReportV2, report_path: &Path) {
    let report_arg = shell_arg(report_path);

    lines.push(String::new());
    lines.push("다음 단계:".to_string());

    if report.summary.deletion_candidates > 0 {
        lines.push(format!("- 정리 미리보기: kratos clean {report_arg}"));
    }
    lines.push(format!(
        "- 공유용 Markdown: kratos report {report_arg} --format md"
    ));
}

fn push_markdown_impact(lines: &mut Vec<String>, report: &ReportV2, report_path: &Path) {
    let impact = impact_summary(report);
    let report_arg = shell_arg(report_path);

    lines.push("## 영향".to_string());
    lines.push(String::new());
    lines.push(format!("- {}", impact.headline));
    lines.push(format!("- 다음 권장 작업: {}", impact.best_next_move));
    if report.summary.deletion_candidates > 0 {
        lines.push(format!("- 정리 미리보기: `kratos clean {report_arg}`"));
    }
    lines.push(format!(
        "- Markdown 갱신: `kratos report {report_arg} --format md`"
    ));
    lines.push(String::new());
}

fn append_deletion_preview(lines: &mut Vec<String>, report: &ReportV2) {
    if report.findings.deletion_candidates.is_empty() {
        return;
    }

    lines.push(String::new());
    lines.push("상위 정리 후보:".to_string());

    for item in report.findings.deletion_candidates.iter().take(5) {
        lines.push(format!(
            "- {} (신뢰도 {}, {})",
            display_path(report, &item.file),
            item.confidence,
            display_known_reason(&item.reason)
        ));
    }

    if report.findings.deletion_candidates.len() > 5 {
        lines.push(format!(
            "- ...외 {}개",
            report.findings.deletion_candidates.len() - 5
        ));
    }
}

fn display_path(report: &ReportV2, path: &Path) -> String {
    match path.strip_prefix(&report.root) {
        Ok(relative) if !relative.as_os_str().is_empty() => path_to_string(relative),
        _ => path_to_string(path),
    }
}

fn shell_arg(path: &Path) -> String {
    let raw = path_to_string(path);
    if raw
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "/._-".contains(character))
    {
        raw
    } else {
        format!("'{}'", raw.replace('\'', "'\\''"))
    }
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
        lines.push(format!("- ...외 {}개", items.len() - 5));
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
        lines.push("- 없음".to_string());
        lines.push(String::new());
        return;
    }

    for item in items {
        lines.push(format!("- {}", render(item)));
    }

    lines.push(String::new());
}

fn display_title(title: &str) -> &str {
    match title {
        "Kratos scan complete." => "Kratos 스캔 완료.",
        "Kratos report." => "Kratos 리포트.",
        other => other,
    }
}

pub fn display_known_reason(reason: &str) -> &str {
    match reason {
        "Component-like module has no inbound references." => {
            "컴포넌트로 보이는 모듈에 참조가 없습니다."
        }
        "Route-like module is not connected to any router entry." => {
            "라우트로 보이는 모듈이 어떤 라우터 진입점에도 연결되지 않았습니다."
        }
        "Module has no inbound references and is not treated as an entrypoint." => {
            "모듈에 참조가 없고 진입점으로 취급되지 않습니다."
        }
        other => other,
    }
}
