use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use kratos_core::analyze::analyze_project;
use kratos_core::report::serialize_report_pretty;
use kratos_core::report_format::format_summary_report;
use kratos_core::{KratosError, KratosResult};

use super::{
    canonicalize_scan_args, resolve_input_path, write_output, CommandSpec,
    DEFAULT_REPORT_RELATIVE_PATH,
};

pub const NAME: &str = "scan";
pub const SPEC: CommandSpec = CommandSpec {
    name: NAME,
    summary: "Analyze a codebase and save the latest report.",
    usage: &["kratos scan [root] [--output path] [--no-write] [--json]"],
};

#[derive(Debug, Parser)]
#[command(disable_help_flag = true, disable_version_flag = true)]
struct ScanArgs {
    #[arg(allow_hyphen_values = true)]
    root: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    output: Option<String>,
    #[arg(long)]
    no_write: bool,
    #[arg(long)]
    json: bool,
}

pub fn run(args: &[String], stdout: &mut dyn Write) -> KratosResult<i32> {
    let args = parse_args(args)?;
    let cwd = std::env::current_dir()?;
    let root = match args.root.as_deref() {
        Some(raw) => resolve_input_path(&cwd, raw),
        None => cwd,
    };
    let uses_default_output_path = args.output.is_none();
    let output_path = resolve_output_path(&root, args.output.as_deref());
    let report = analyze_project(&root)?;
    let serialized = serialize_report_pretty(&report)?;

    if !args.no_write {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output_path, format!("{serialized}\n"))?;
    }

    if args.json {
        write_output(stdout, &serialized)?;
        return Ok(0);
    }

    let summary = format_scan_summary(
        &report,
        &output_path,
        args.no_write,
        uses_default_output_path,
    )?;
    write_output(stdout, &summary)?;
    Ok(0)
}

fn parse_args(args: &[String]) -> KratosResult<ScanArgs> {
    let canonical = canonicalize_scan_args(args)?;
    ScanArgs::try_parse_from(std::iter::once(NAME).chain(canonical.iter().map(String::as_str)))
        .map_err(|error| KratosError::Config(error.to_string().trim().to_string()))
}

fn format_scan_summary(
    report: &kratos_core::model::ReportV2,
    output_path: &Path,
    no_write: bool,
    uses_default_output_path: bool,
) -> KratosResult<String> {
    let summary = format_summary_report(report, output_path, "Kratos scan complete.")?;
    let mut lines = Vec::new();

    for line in summary.lines() {
        if no_write {
            if line.starts_with("저장된 리포트: ") {
                lines
                    .push("리포트 저장: --no-write 때문에 파일을 생성하지 않았습니다.".to_string());
                continue;
            }

            if line.starts_with("- 정리 미리보기: ") || line.starts_with("- 공유용 Markdown: ")
            {
                continue;
            }
        }

        lines.push(line.to_string());

        if no_write && line == "다음 단계:" {
            lines.push(
                "- 리포트가 필요한 clean/report 작업은 기본 쓰기 모드로 다시 실행하거나 --output path를 지정하세요."
                    .to_string(),
            );
        }

        if !no_write && uses_default_output_path && line.starts_with("저장된 리포트: ") {
            lines.push(
                "쓰기 안내: 기본 리포트 경로 .kratos/latest-report.json는 체크아웃을 dirty하게 만들 수 있습니다. .gitignore에 .kratos/를 추가하거나 --output 또는 --no-write를 사용하세요."
                    .to_string(),
            );
        }

        if !no_write && line.starts_with("- 정리 미리보기: kratos clean ") {
            let report_arg = line
                .strip_prefix("- 정리 미리보기: kratos clean ")
                .unwrap_or_default();
            lines.push(format!(
                "- npx로 실행 중이라면: npx @jeremyfellaz/kratos clean {report_arg}"
            ));
        }

        if !no_write && line.starts_with("- 공유용 Markdown: kratos report ") {
            let report_args = line
                .strip_prefix("- 공유용 Markdown: kratos report ")
                .unwrap_or_default();
            lines.push(format!(
                "- npx로 실행 중이라면: npx @jeremyfellaz/kratos report {report_args}"
            ));
        }
    }

    Ok(lines.join("\n"))
}

fn resolve_output_path(root: &Path, output_flag: Option<&str>) -> PathBuf {
    match output_flag {
        Some(raw) => resolve_input_path(root, raw),
        None => resolve_input_path(root, DEFAULT_REPORT_RELATIVE_PATH),
    }
}
