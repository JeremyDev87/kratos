# Rust Migration Overview Plan

## Objective

Kratos를 현재의 Node/JS 구현에서 Rust 중심 구현으로 전환한다. 최종 사용자 경험은 계속 `npm install`, `npx kratos scan`, `npx kratos report`, `npx kratos clean`을 유지하고, 내부 런타임만 `Rust core + Rust CLI + Node launcher` 구조로 교체한다. 이번 전환은 제품 재작성이지 기능 확장이 아니므로, 현재 README에 문서화된 MVP 기능만 Rust로 다시 구현한다.

## Conflict Surface

- `package.json`
- `.gitignore`
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `scripts/release-plan.mjs`
- `scripts/classify-npm-lookup-error.mjs`
- `scripts/classify-npm-publish-error.mjs`
- `README.md`
- `README.en.md`
- `README.es.md`
- `README.ja.md`
- `README.zh-CN.md`
- `CONTRIBUTING.md`
- `src/cli.js`
- `src/commands/scan.js`
- `src/commands/report.js`
- `src/commands/clean.js`
- `src/lib/*.js`
- `test/cli-help.test.js`
- `test/kratos.test.js`

## Rules

- 최종 공개 명령은 계속 `scan`, `report`, `clean`만 유지한다.
- config 파일 이름은 계속 `kratos.config.json`을 사용한다.
- 현재 JS 구현은 baseline으로 유지하고, cutover 전까지 부분적으로 고치지 않는다.
- `package.json`은 `PR 6A. Root Package Cutover And JS Runtime Removal` 전까지 건드리지 않고, 이후에는 stable promotion용 `Release Prep 8A. Version-Only Stable Promotion`에서만 다시 수정한다.
- `.github/workflows/ci.yml`와 `.github/workflows/release.yml`는 `PR 5B. CI And Release Rewrite For Rust Artifacts`만 수정한다.
- 다국어 README와 `CONTRIBUTING.md`는 `PR 7A. Docs Sync And Alpha Release Readiness` 전까지 수정하지 않는다.
- Rust crate 구조는 `Cargo.toml` 워크스페이스 + `crates/kratos-core`, `crates/kratos-cli`, `crates/kratos-node`로 고정한다.
- Rust parser는 `Oxc`로 고정한다. 다른 parser stack으로 갈아타는 작업은 본 계획 범위 밖이다.
- npm 배포 경험은 유지하되, `cargo install`은 공식 배포 경로로 문서화하지 않는다.
- 보고서 스키마는 Rust cutover 이후 `schemaVersion: 2`로 단일화한다. `clean`은 v2 report만 읽고 v1은 명시적으로 거부한다.

## Final Repo Shape

- `Cargo.toml`
- `Cargo.lock`
- `rust-toolchain.toml`
- `crates/kratos-core/`
- `crates/kratos-cli/`
- `crates/kratos-node/`
- `bin/kratos.js`
- `fixtures/parity/`
- `src/`는 cutover 시점에 제거

## Final Public Interfaces

- 유지:
  - `kratos scan [root] [--output path] [--json]`
  - `kratos report [report-path-or-root] [--format summary|json|md]`
  - `kratos clean [report-path-or-root] [--apply]`
  - 기본 report 경로 `.kratos/latest-report.json`
- 변경:
  - report JSON은 `schemaVersion: 2`
  - top-level `version`과 `root` 제거
  - `project.root`, `project.configPath`, `engine.*`, `graph.modules` 추가

## Phase Map

### Phase 1. Rust Foundation

- `PR 1A. Rust Workspace Skeleton And Shared Domain Types`
- `PR 1B. JS Parity Fixture Capture`

### Phase 2. Core Inputs And Parser

- `PR 2A. Config Discovery And Resolve In Rust`
- `PR 2B. Oxc Parser And Symbol Extraction`

### Phase 3. Analysis Surface

- `PR 3A. Graph Analysis And Report V2`
- `PR 3B. Clean Safety And Report V2 Consumption`

### Phase 4. Rust CLI

- `PR 4A. Rust CLI For Scan Report Clean`

### Phase 5. Packaging And Automation

- `PR 5A. NAPI Wrapper And JS Launcher`
- `PR 5B. CI And Release Rewrite For Rust Artifacts`

### Phase 6. Big-Bang Cutover

- `PR 6A. Root Package Cutover And JS Runtime Removal`

### Phase 7. Docs And Verification

- `PR 7A. Docs Sync And Alpha Release Readiness`
- `Verification Gate 7B. Full Multi-Platform Verification`

### Phase 8. Stable Release Promotion

- `Release Prep 8A. Version-Only Stable Promotion`

## Preflight Requirements

- 로컬 개발자는 Rust stable toolchain과 `cargo`를 설치해야 한다.
- Node 18 이상은 계속 필요하다. npm wrapper와 parity capture script가 Node를 사용한다.
- 플랫폼별 addon 패키지 이름과 target 매핑은 `PR 5A`에서 확정하고, `PR 5B`는 `PR 5A` merge 이후 그 이름을 그대로 사용한다.
- 첫 Rust alpha 릴리스용 root package 버전은 `PR 6A`에서 `0.2.0-alpha.1`로 맞춘다.
- `0.2.0` stable 승격은 alpha 검증 뒤 별도의 version-only release-prep commit에서 수행한다.

## Done Criteria

- Rust 경로에서 `scan`, `report`, `clean`이 현재 README의 MVP 범위를 동작시킨다.
- npm wrapper가 5개 공식 target에서 Rust 런타임을 로드할 수 있다.
- `schemaVersion: 2` report가 일관되게 생성되고 `clean`이 이를 소비한다.
- cutover 후 npm tarball에 `src/` 런타임이 포함되지 않는다.
- 문서와 실제 릴리스 절차가 일치한다.

## Out Of Scope

- HTML report
- monorepo/workspace support
- baseline diff
- watch mode
- interactive clean preview
- confidence threshold clean
- React.lazy / `next/dynamic`
- suppression UX
- incremental cache
