# Rust Foundation And Parity Plan

## Objective

Rust 전환의 첫 단계로 workspace와 crate 구조를 고정하고, 이후 PR들이 같은 파일을 두고 충돌하지 않도록 모듈 뼈대를 미리 심는다. 동시에 현재 JS 구현의 출력물을 parity baseline으로 고정해, 이후 Rust 구현이 무엇을 맞춰야 하는지 low-context 모델도 재구성 없이 이해할 수 있게 만든다.

## Conflict Surface

- `Cargo.toml`
- `Cargo.lock`
- `rust-toolchain.toml`
- `crates/kratos-core/Cargo.toml`
- `crates/kratos-core/src/lib.rs`
- `crates/kratos-cli/Cargo.toml`
- `crates/kratos-cli/src/main.rs`
- `crates/kratos-node/Cargo.toml`
- `crates/kratos-node/src/lib.rs`
- `scripts/capture-parity-fixtures.mjs`
- `fixtures/parity/**`

## Rules

- `PR 1A`는 이후 PR들이 `lib.rs`와 crate manifest를 다시 수정하지 않도록 placeholder module tree를 한 번에 만든다.
- `PR 1A`는 실제 기능을 구현하지 않는다. 모든 공개 함수는 compile 가능한 stub 또는 명시적 `NotImplemented` 오류만 반환한다.
- `PR 1B`는 현재 JS 구현을 기준으로 baseline을 캡처한다. Rust 코드를 호출하거나 Rust 결과를 섞지 않는다.
- `package.json`, workflows, README는 이 plan family에서 수정하지 않는다.

## PR 1A. Rust Workspace Skeleton And Shared Domain Types

### Target Files

- `Cargo.toml`
- `Cargo.lock`
- `rust-toolchain.toml`
- `crates/kratos-core/Cargo.toml`
- `crates/kratos-core/src/lib.rs`
- `crates/kratos-core/src/error.rs`
- `crates/kratos-core/src/model.rs`
- `crates/kratos-core/src/report.rs`
- `crates/kratos-core/src/config.rs`
- `crates/kratos-core/src/jsonc.rs`
- `crates/kratos-core/src/discover.rs`
- `crates/kratos-core/src/resolve.rs`
- `crates/kratos-core/src/entrypoints.rs`
- `crates/kratos-core/src/analyze.rs`
- `crates/kratos-core/src/clean.rs`
- `crates/kratos-core/src/parser/mod.rs`
- `crates/kratos-core/src/parser/adapter.rs`
- `crates/kratos-core/src/parser/imports.rs`
- `crates/kratos-core/src/parser/exports.rs`
- `crates/kratos-core/src/parser/unused_imports.rs`
- `crates/kratos-cli/Cargo.toml`
- `crates/kratos-cli/src/main.rs`
- `crates/kratos-cli/src/commands/mod.rs`
- `crates/kratos-cli/src/commands/scan.rs`
- `crates/kratos-cli/src/commands/report.rs`
- `crates/kratos-cli/src/commands/clean.rs`
- `crates/kratos-node/Cargo.toml`
- `crates/kratos-node/src/lib.rs`

### Steps

1. 루트 `Cargo.toml`을 workspace manifest로 만들고 member를 `crates/kratos-core`, `crates/kratos-cli`, `crates/kratos-node`로 고정한다.
2. `rust-toolchain.toml`은 stable channel을 고정한다. nightly 전제는 두지 않는다.
3. `crates/kratos-core/src/lib.rs`에서 앞으로 사용할 모든 public module을 한 번에 선언한다.
4. `model.rs`에는 아래 typed model을 먼저 정의한다.
   - `ProjectConfig`
   - `ModuleRecord`
   - `ImportRecord`
   - `ExportRecord`
   - `FindingSet`
   - `ReportV2`
   - `SummaryCounts`
5. `report.rs`에는 `ReportV2` 직렬화용 helper와 summary/markdown formatter용 function signature만 먼저 만든다.
6. `error.rs`에는 `KratosError` enum을 만들고 최소 variant를 아래로 고정한다.
   - `Io`
   - `Json`
   - `Config`
   - `InvalidReportVersion`
   - `NotImplemented`
7. `crates/kratos-cli/src/main.rs`와 `src/commands/*.rs`는 command/flag shape만 compile 되게 잡고 실제 로직은 `NotImplemented` 오류로 연결한다.
8. `crates/kratos-node/src/lib.rs`에는 최종 `run_cli(args: Vec<String>) -> Result<i32, KratosError>` signature만 먼저 잡는다.

### Behavior Constraints

- 이 PR의 목적은 구조 seed다. 현재 JS 실행 경로를 바꾸지 않는다.
- 이후 PR에서 `crates/kratos-core/src/lib.rs`와 `crates/kratos-cli/src/commands/mod.rs`를 다시 만지지 않도록 필요한 module 선언을 여기서 끝낸다.
- placeholder implementation은 panic보다 명시적 오류를 선호한다.

### Validation

- `cargo test`가 가능하면 workspace가 compile 가능한 상태인지 확인한다.
- compile 검증이 불가능한 환경이라면, 문서에 명시된 파일과 module tree가 빠짐없이 생성되었는지 수동 검토한다.

## PR 1B. JS Parity Fixture Capture

### Target Files

- `scripts/capture-parity-fixtures.mjs`
- `fixtures/parity/README.md`
- `fixtures/parity/manifest.json`
- `fixtures/parity/demo-app/latest-report.v1.json`
- `fixtures/parity/demo-app/report-summary.txt`
- `fixtures/parity/demo-app/report-markdown.md`

### Steps

1. `fixtures/parity/manifest.json`에 parity fixture 목록을 명시한다. 첫 fixture는 반드시 `fixtures/demo-app`만 포함한다.
2. `scripts/capture-parity-fixtures.mjs`는 현재 JS CLI를 호출해 아래 세 결과를 저장한다.
   - `scan`이 생성한 JSON report
   - `report --format summary` 출력
   - `report --format md` 출력
3. absolute path는 fixture 상대 placeholder로 정규화한다.
   - project root는 `<ROOT>`
   - report path는 `<REPORT>`
   - 경로 구분자는 `/`로 고정
4. `fixtures/parity/README.md`에 snapshot을 다시 캡처하는 정확한 명령을 적는다.

### Behavior Constraints

- baseline은 현재 JS 구현을 고정하는 자료다. 사람이 손으로 편집하지 않는다.
- summary와 markdown은 CLI 출력 그대로 저장하되, 경로만 placeholder로 정규화한다.
- 새 fixture를 임의로 추가하지 않는다. Phase 1에서는 `demo-app` 하나만 유지한다.

### Validation

- `node ./scripts/capture-parity-fixtures.mjs`가 3개 산출물을 갱신하는지 확인한다.
- `fixtures/parity/demo-app/*`가 현재 README의 예시와 모순되지 않는지 확인한다.

## Done Criteria

- Rust workspace skeleton과 typed model이 고정됐다.
- 이후 phase가 같은 manifest/lib root를 반복 편집할 필요가 없게 되었다.
- JS baseline 출력이 repo 안에 정규화된 fixture로 저장되었다.

## Out Of Scope

- 실제 Rust 분석 로직 구현
- npm wrapper 배선
- CI/release 변경
