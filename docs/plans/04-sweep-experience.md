# Kratos Sweep Experience Plan

## Objective

`clean`의 deletion candidate를 사용자가 하나씩 판단할 수 있는 guided cleanup UX로 연결한다. Pure clean preview planner는 이미 `crates/kratos-core/src/clean_preview.rs`에 존재하므로, 이 문서는 남은 `kratos sweep` CLI만 다룬다.

## Current Baseline

- `clean`은 기본 dry-run이며 `--apply`가 있을 때만 삭제한다.
- `clean --min-confidence`와 `thresholds.cleanMinConfidence`가 동작한다.
- suppression은 `kratos.config.json`과 `.kratos/suppressions.json`에서 읽힌다.
- clean preview helper는 삭제를 수행하지 않는 core helper로 분리되어 있다.

## Rules

- `sweep`는 report를 읽어 동작한다. 자체적으로 자동 rescan하지 않는다.
- interactive mode는 `stdin.read_line()` 기반 prompt만 사용한다.
- `sweep`에서 suppression을 저장할 때는 `.kratos/suppressions.json`만 쓴다.
- `sweep`는 threshold와 suppression을 모두 반영한 뒤 preview 순서를 결정한다.
- 사용자의 명시적 선택 없이 파일 삭제가 일어나면 안 된다.

## PR WOW-5A. kratos sweep Interactive CLI

### Target Files

- `crates/kratos-cli/src/commands/mod.rs`
- `new command file: crates/kratos-cli/src/commands/sweep.rs`
- `new CLI spec: crates/kratos-cli/tests/sweep_cli.rs`
- `crates/kratos-cli/tests/support/mod.rs`
- `crates/kratos-cli/tests/support/cli.rs`

### Command Contract

- usage:
  - `kratos sweep [report-path-or-root] [--min-confidence value] [--yes]`
- prompt actions:
  - `y`: accept current file for deletion
  - `n`: skip current file
  - `s`: write suppression rule and skip current file
  - `a`: accept all remaining eligible files
  - `q`: stop and keep remaining files untouched

### Steps

1. `clean`과 같은 input resolution helper를 써서 report를 읽는다.
2. preview helper 결과를 하나씩 보여주고 line prompt를 받는다.
3. `s`를 누르면 exact suppression rule을 `.kratos/suppressions.json`에 append하고 현재 item은 삭제 후보에서 제외한다.
4. prompt loop가 끝난 뒤 accepted item이 있으면 그 path들만 delete helper에 넘긴다.
5. `--yes`는 interactive prompt 없이 threshold/suppression을 통과한 모든 item을 accept한다.
6. summary에는 `deleted`, `suppressed`, `skipped`, `below threshold`, `remaining untouched`를 모두 보여준다.

## Done Criteria

- [ ] `sweep`가 interactive mode와 `--yes` mode를 모두 가진다.
- [ ] suppression write가 `.kratos/suppressions.json`에 누적된다.
- [ ] `sweep`가 report input만 사용하고 implicit rescan을 하지 않는다.
- [ ] no-selection path에서 파일이 삭제되지 않는 회귀 테스트가 있다.

## Out Of Scope

- full-screen terminal UI
- inline diff patch rendering
- automatic git commit
- delete undo stack
- multi-user lock file
