# Kratos Report Presentation Plan

## Objective

현재 `kratos diff`와 `report --format summary|json|md`는 이미 구현되어 있다. 이 문서는 남은 presentation 작업인 self-contained HTML report만 다룬다.

## Current Baseline

- `crates/kratos-core/src/report_format.rs`가 summary/markdown formatter를 소유한다.
- `crates/kratos-core/src/report_diff.rs`와 `crates/kratos-cli/src/commands/diff.rs`가 `kratos diff`를 제공한다.
- `crates/kratos-cli/src/commands/report.rs`는 현재 `summary|json|md`만 허용한다.

## Rules

- HTML output은 단일 파일이며 CSS/JS asset을 외부 CDN에서 가져오지 않는다.
- `report --format html`은 stdout에 HTML 본문을 쓰지 않고 파일을 쓴다.
- `summary|json|md` 출력은 기존 path와 text 의미를 유지한다.
- snapshot test는 `generatedAt`과 absolute path를 placeholder로 normalize한다.

## PR WOW-4A. Self-Contained HTML Report Renderer

### Target Files

- `crates/kratos-core/src/lib.rs`
- `crates/kratos-core/src/report_format.rs`
- `new helper file: crates/kratos-core/src/report_html.rs`
- `crates/kratos-cli/src/commands/report.rs`
- `new core spec: crates/kratos-core/tests/report_html.rs`
- `new CLI spec: crates/kratos-cli/tests/report_html_cli.rs`
- `crates/kratos-cli/tests/support/mod.rs`
- `crates/kratos-cli/tests/support/cli.rs`

### Helper Contract

- HTML renderer input:
  - parsed `ReportV2`
  - report file path
  - optional title string
- HTML renderer output:
  - complete HTML string with inline CSS and inline JS

### Steps

1. summary cards, finding tabs, search input, section anchors를 가진 self-contained HTML layout를 만든다.
2. inline JS는 client-side filter/search만 수행하고 network request는 하지 않는다.
3. `report --format html`에 `--output <path>`를 추가한다.
4. `--format html`에서 `--output`이 없으면 입력이 file path일 때 sibling `.html`, root일 때 `.kratos/latest-report.html`을 기본 경로로 사용한다.
5. command는 HTML file을 쓴 뒤 saved path를 stdout으로 한 줄 출력하고 종료한다.
6. `summary|json|md` 기존 동작을 CLI regression test로 보호한다.

## Done Criteria

- [ ] HTML report가 외부 dependency 없이 단일 파일로 생성된다.
- [ ] `report --format html`이 기본 output path와 명시적 `--output`을 모두 지원한다.
- [ ] `report --format summary|json|md` 기존 동작이 회귀 테스트로 보호된다.

## Out Of Scope

- git commit/branch 자동 비교
- browser 자동 열기
- external chart library
- module graph visualization canvas
- HTML theme customization
