# Kratos Active Roadmap Overview

## Current Baseline

Kratos는 현재 Rust core/CLI와 npm launcher 기반으로 동작한다. 공개 npm package 이름은 `@jeremyfellaz/kratos`이고, CLI binary 이름은 `kratos`다.

현재 `master`에 반영된 주요 기능:

- `scan`, `report`, `diff`, `clean`
- report `summary|json|md`
- report schema `schemaVersion: 2`
- `React.lazy` / `next/dynamic` 기반 dynamic usage 인식
- `kratos.config.json` 및 `.kratos/suppressions.json` suppression
- `thresholds.cleanMinConfidence` 및 `clean --min-confidence`
- `.gitignore` + `ignorePatterns` 기반 source discovery 제외
- `clean` dry-run preview planner

## Active Objective

남은 roadmap은 현재 CLI 흐름을 유지하면서 다음 사용자 경험을 추가하는 것이다.

- self-contained HTML report
- line-oriented `kratos sweep`
- workspace manifest 기반 multi-root scan
- incremental analysis cache
- polling `kratos watch`
- 최종 README/CONTRIBUTING 동기화와 전체 검증

릴리스 workflow 재설계, npm publish, release tag 생성은 이 roadmap 범위 밖이다.

## Rules

- 기존 `scan`, `report`, `diff`, `clean` 입력 의미와 기본 동작은 유지한다.
- 새 공개 명령은 `sweep`, `watch`만 남은 범위로 본다.
- `report`는 기존 `summary|json|md`를 유지하면서 `html`을 추가한다.
- report JSON의 `schemaVersion`은 계속 `2`를 유지한다.
- human-authored config는 계속 `kratos.config.json`을 사용한다.
- machine-authored suppression은 `.kratos/suppressions.json`에 저장한다. `sweep`는 이 파일만 자동으로 쓴다.
- `watch`는 OS 전용 file watcher dependency 대신 polling loop로 구현한다.
- workspace 지원은 `package.json#workspaces`와 `pnpm-workspace.yaml` 기반 discovery만 다룬다.
- README와 `CONTRIBUTING.md`는 새 기능이 실제로 merge된 뒤 최종 문서 PR에서 한 번에 동기화한다.

## Active Phase Map

### Phase A. Report Presentation

- `PR WOW-4A. Self-Contained HTML Report Renderer`

### Phase B. Guided Cleanup

- `PR WOW-5A. kratos sweep Interactive CLI`

### Phase C. Workspace Scale

- `PR WOW-6A. Workspace Manifest Detection And Multi-Root Scan`

### Phase D. Incremental Performance

- `PR WOW-7A. Incremental Analysis Cache`

### Phase E. Watch Loop

- `PR WOW-8A. kratos watch Polling Loop With Cached Delta Output`

### Phase F. Docs And Verification

- `PR WOW-9A. Docs Sync And Demo Fixtures`
- `Verification Gate WOW-9B. Full Wow Feature Verification`

## Shared Serial Lanes

- `crates/kratos-cli/src/commands/mod.rs`
  - `sweep`와 `watch` command 등록이 모두 이 파일을 건드린다. 같은 checkout에서는 순차 처리한다.
- `crates/kratos-core/src/lib.rs`
  - `report_html`, `workspaces`, `cache` module export가 모두 이 파일에 모인다.
- `crates/kratos-core/src/config.rs`
  - workspace discovery와 cache invalidation fingerprint가 같은 config surface를 공유한다.
- `README*`, `CONTRIBUTING.md`
  - landed UX 기준으로 최종 phase에서 한 번에 갱신한다.

## Completed And Removed From Active Planning

- Rust migration planning docs under `docs/plan/`
- JS-era CLI help modularization plan
- report formatter ownership consolidation
- CLI test support extraction
- dynamic usage awareness
- suppression schema and finding filter
- clean confidence threshold
- report diff engine and `kratos diff`
- clean preview planner
- `.gitignore` ingestion and ignore-pattern scan behavior

## Out Of Scope

- release workflow 수정
- npm publish/tag 실행
- browser auto-open
- full-screen TUI
- new schema version rollout
