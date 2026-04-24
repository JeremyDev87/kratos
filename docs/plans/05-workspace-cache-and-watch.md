# Kratos Workspace Cache And Watch Plan

## Objective

Kratos를 단일 프로젝트 정리 도구에서 monorepo와 반복 실행에도 견디는 도구로 확장한다. 이 plan family는 workspace manifest 기반 root scan, incremental cache, 그리고 polling `watch` loop를 순차적으로 추가한다. 단일 package repo의 현재 동작은 유지되어야 한다.

## Current Baseline

- `scan`은 현재 단일 root를 분석하고 `.kratos/latest-report.json`을 쓴다.
- `diff` helper는 이미 존재하므로 `watch` delta 출력에서 재사용할 수 있다.
- `.gitignore`, `ignorePatterns`, suppression, clean threshold는 이미 source discovery와 분석 경로에 반영되어 있다.
- workspace metadata, parser cache, `scan --no-cache`, `watch` command는 아직 없다.

## Conflict Surface

- `crates/kratos-core/src/lib.rs`
- `crates/kratos-core/src/model.rs`
- `crates/kratos-core/src/config.rs`
- `crates/kratos-core/src/discover.rs`
- `crates/kratos-core/src/resolve.rs`
- `crates/kratos-core/src/analyze.rs`
- `crates/kratos-core/src/report.rs`
- `crates/kratos-core/Cargo.toml`
- `crates/kratos-cli/src/commands/scan.rs`
- `crates/kratos-cli/src/commands/mod.rs`

## Rules

- workspace가 없는 repo는 지금과 같은 single-root path를 그대로 사용한다.
- workspace discovery는 manifest에 명시된 package list만 따른다.
- cache는 advisory optimization이다. cache가 깨져도 full scan으로 fallback 해야 한다.
- `watch`는 `report_diff`를 재사용해 delta를 보여준다. 별도 delta schema를 만들지 않는다.
- cache file과 generated suppression file은 둘 다 `.kratos/` 아래에 저장한다.

## Slice 1. PR WOW-6A. Workspace Manifest Detection And Multi-Root Scan

### Target Files

- `crates/kratos-core/src/lib.rs`
- `crates/kratos-core/src/model.rs`
- `crates/kratos-core/src/config.rs`
- `crates/kratos-core/src/discover.rs`
- `crates/kratos-core/src/resolve.rs`
- `crates/kratos-core/src/analyze.rs`
- `crates/kratos-core/src/report.rs`
- `crates/kratos-core/Cargo.toml`
- `new helper file: crates/kratos-core/src/workspaces.rs`
- `crates/kratos-core/tests/config_and_discovery.rs`
- `new core spec: crates/kratos-core/tests/analyze_workspaces.rs`

### Helper Contract

- supported manifest inputs:
  - `package.json.workspaces` array
  - `package.json.workspaces.packages`
  - `pnpm-workspace.yaml` `packages`
- per-module metadata:
  - `workspaceRoot`
  - `workspaceName`

### Steps

1. root 아래 workspace manifest를 읽고 package root list를 계산하는 helper를 만든다.
2. source discovery는 workspace root별로 실행하되, existing `ignore`/`roots` 규칙을 계속 적용한다.
3. module record에 `workspaceRoot`, `workspaceName` optional field를 추가한다.
4. import resolution은 importer와 가장 가까운 workspace package.json dependency map을 먼저 사용하고, 없으면 root package.json으로 fallback 한다.
5. report serialization은 optional workspace metadata를 추가하되 `schemaVersion`은 유지한다.
6. `pnpm-workspace.yaml` 파싱을 위해 필요한 최소 dependency 추가가 있으면 `crates/kratos-core/Cargo.toml`에서만 처리한다.

## Slice 2. PR WOW-7A. Incremental Analysis Cache

### Target Files

- `crates/kratos-core/src/lib.rs`
- `crates/kratos-core/src/analyze.rs`
- `crates/kratos-core/src/config.rs`
- `crates/kratos-core/src/report.rs`
- `crates/kratos-core/Cargo.toml`
- `crates/kratos-cli/src/commands/scan.rs`
- `new helper file: crates/kratos-core/src/cache.rs`
- `new core spec: crates/kratos-core/tests/cache_reuse.rs`
- `new CLI spec: crates/kratos-cli/tests/scan_cache_cli.rs`

### Helper Contract

- cache file path: `.kratos/cache-v1.json`
- cache key inputs:
  - file content hash
  - normalized relative path
  - parser/cache format version
  - config fingerprint
  - workspace manifest fingerprint
  - suppression file fingerprint

### Steps

1. file별 parsed imports/exports/unused-import result를 cache entry로 저장한다.
2. unchanged file은 parser 단계만 cache reuse하고, final graph analysis는 항상 current snapshot 기준으로 다시 계산한다.
3. config, workspace manifest, suppression file이 바뀌면 관련 entry를 모두 invalidate 한다.
4. `scan`에 `--no-cache` flag를 추가해 read/write를 모두 끈다.
5. cache read/write 실패는 warning 없이 full scan fallback으로 처리한다.

## Slice 3. PR WOW-8A. kratos watch Polling Loop With Cached Delta Output

### Target Files

- `crates/kratos-cli/src/commands/mod.rs`
- `new command file: crates/kratos-cli/src/commands/watch.rs`
- `new CLI spec: crates/kratos-cli/tests/watch_cli.rs`
- `crates/kratos-cli/tests/support/mod.rs`
- `crates/kratos-cli/tests/support/cli.rs`

### Command Contract

- usage:
  - `kratos watch [root] [--debounce-ms value] [--min-confidence value] [--once] [--no-cache]`

### Steps

1. 첫 실행에서 root를 scan하고 `.kratos/latest-report.json`을 쓴다.
2. 이후 polling loop는 file mtimes 또는 directory metadata를 보고 변경이 있으면 scan을 다시 실행한다.
3. 이전 report와 새 report를 `kratos diff`와 같은 helper로 비교해 one-line delta summary를 출력한다.
4. `--once`는 polling loop를 돌지 않고 scan 1회 + report write 후 종료한다.
5. `--no-cache`는 cache reuse를 끄고 full scan path를 강제한다.
6. Ctrl-C를 받으면 partial file write 없이 현재 loop를 마치고 종료한다.

## Done Criteria

- [ ] workspace manifest가 있는 root에서 per-workspace metadata를 가진 report가 생성된다.
- [ ] unchanged file parser 결과가 cache reuse된다.
- [ ] `scan --no-cache`와 `watch --no-cache`가 cache를 우회한다.
- [ ] `watch --once`가 interactive loop 없이 scan 1회만 수행한다.
- [ ] single-root project의 기존 `scan` summary count와 output path가 바뀌지 않는다.

## Out Of Scope

- Nx/Turbo graph analysis
- OS-native file watcher
- cross-process daemon
- distributed cache
- browser live reload
