# Rust CLI And Node Packaging Plan

## Objective

`kratos-core` 위에 실제 Rust CLI를 올리고, npm 사용 경험을 유지하기 위한 Node launcher와 NAPI wrapper를 준비한다. 이 plan family는 사용자 명령 UX와 배포 껍데기를 다루며, cutover 전에 기존 npm `bin`을 바꾸지는 않는다.

## Conflict Surface

- `crates/kratos-cli/src/main.rs`
- `crates/kratos-cli/src/commands/scan.rs`
- `crates/kratos-cli/src/commands/report.rs`
- `crates/kratos-cli/src/commands/clean.rs`
- `crates/kratos-cli/tests/**`
- `crates/kratos-node/src/lib.rs`
- `bin/kratos.js`
- `test/npm-launcher.test.js`
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`

## Rules

- `PR 4A`는 `crates/kratos-cli/**/*`만 수정한다. Node wrapper나 workflow는 같이 건드리지 않는다.
- `PR 5A`는 `crates/kratos-node/**/*`, `bin/kratos.js`, `test/npm-launcher.test.js`를 수정한다.
- `PR 5A`에서 NAPI wrapper가 CLI 로직을 재사용하기 위해 필요한 최소 범위의 `crates/kratos-cli/src/lib.rs` 추출과 `crates/kratos-cli/src/main.rs` thin-wrapper 변경은 허용한다.
- `PR 5B`는 `PR 5A`가 merge되어 addon package name과 target 매핑이 고정된 뒤에만 시작한다.
- `PR 5C`는 `PR 5B`가 merge되어 release lane이 정리된 뒤에만 시작한다.
- `PR 5B`와 `PR 5C`는 workflow 파일만 수정한다. `package.json`은 `PR 6A`까지 건드리지 않는다.
- unknown command/help/error wording은 현재 JS CLI와 최대한 동일한 읽기 경험을 유지한다.

## PR 4A. Rust CLI For Scan Report Clean

### Target Files

- `crates/kratos-cli/src/main.rs`
- `crates/kratos-cli/src/commands/scan.rs`
- `crates/kratos-cli/src/commands/report.rs`
- `crates/kratos-cli/src/commands/clean.rs`
- `crates/kratos-cli/tests/cli_smoke.rs`

### Steps

1. `main.rs`에서 `clap` subcommand 구조를 아래처럼 고정한다.
   - `scan [root] [--output path] [--json]`
   - `report [report-path-or-root] [--format summary|json|md]`
   - `clean [report-path-or-root] [--apply]`
2. no-arg, `help`, `--help`, `-h`는 root help를 출력한다.
3. subcommand에 `--help`가 오면 command-specific help를 출력한다.
4. `scan`은 core scan 함수를 호출해 `.kratos/latest-report.json`에 저장하고, `--json` 없으면 summary를 stdout에 출력한다.
5. `report`는 summary/json/md 포맷을 stdout에 출력한다.
6. `clean`은 dry-run이면 후보 목록과 재실행 안내를 출력하고, `--apply`면 삭제 수를 출력한다.

### Behavior Constraints

- 오류는 `Kratos failed: ...` 형식으로 stderr에 출력한다.
- unknown command는 현재 JS help와 동일한 흐름으로 1 exit code를 반환한다.
- `report --format`은 summary, json, md 외 값이면 에러로 처리한다.

### Validation

- `cli_smoke.rs`에 아래를 포함한다.
  - root help
  - unknown command
  - `scan fixtures/demo-app`
  - `report fixtures/demo-app/.kratos/latest-report.json --format md`
  - `clean fixtures/demo-app/.kratos/latest-report.json`

## PR 5A. NAPI Wrapper And JS Launcher

### Target Files

- `crates/kratos-node/src/lib.rs`
- `crates/kratos-cli/src/lib.rs`
- `crates/kratos-cli/src/main.rs`
- `bin/kratos.js`
- `test/npm-launcher.test.js`

### Steps

1. `crates/kratos-cli/src/lib.rs`에 `run_cli(args: &[String]) -> i32` 공용 실행 함수를 둔다.
2. `crates/kratos-cli/src/main.rs`는 argv 수집 후 공용 실행 함수만 호출하는 thin wrapper로 유지한다.
3. `crates/kratos-node/src/lib.rs`에서 `runCli(args: Vec<String>) -> i32`를 export한다.
4. native layer는 CLI main logic를 다시 구현하지 않고 `kratos-cli`의 공용 실행 함수만 호출한다.
5. `bin/kratos.js`는 아래 순서로 동작한다.
   - `process.argv.slice(2)` 수집
   - platform/arch에 맞는 native addon 패키지 resolve
   - `runCli(args)` 호출
   - return value를 `process.exitCode`에 반영
6. native load 실패나 Rust error는 `Kratos failed: ...` 형식으로 stderr에 출력한다.
7. addon package name은 아래 형식으로 고정한다.
   - `@kratos/darwin-arm64`
   - `@kratos/darwin-x64`
   - `@kratos/linux-x64-gnu`
   - `@kratos/linux-arm64-gnu`
   - `@kratos/win32-x64-msvc`

### Behavior Constraints

- 이 PR은 아직 루트 `package.json`의 `bin`을 바꾸지 않는다.
- launcher는 sync entrypoint만 제공한다. background worker나 daemon은 추가하지 않는다.

### Validation

- `npm-launcher.test.js`에서 missing addon error formatting과 argv forwarding을 검증한다.

## PR 5B. CI And Release Rewrite For Rust Artifacts

### Target Files

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`

### Steps

1. `ci.yml`에 기존 Node matrix verify를 유지하면서 Rust build/test job을 추가한다.
2. Rust job은 stable toolchain 기준으로 `cargo test --workspace`를 실행한다.
3. release workflow는 `PR 5A`에서 고정한 addon package name과 target 매핑을 그대로 사용한다.
4. release workflow는 target artifact build matrix를 먼저 실행하고, 각 target별 addon artifact를 준비한 뒤 root package publish 단계로 넘어가게 순서를 재작성한다.
5. release workflow는 root package를 publish하기 전에 모든 target artifact job 성공을 요구한다.
6. GitHub Release asset에는 target artifact와 root npm tarball을 모두 첨부한다.

### Behavior Constraints

- root package publish가 addon publish보다 먼저 실행되면 안 된다.
- workflow는 addon package naming source of truth를 임의로 새로 만들지 않는다. `PR 5A` 산출물을 그대로 따른다.
- cutover 전이므로 workflow는 기존 Node smoke와 새 Rust smoke를 함께 돌린다.

### Validation

- CI dry-run 관점에서 아래 경로가 모두 문서화된 상태인지 확인한다.
  - Node verify
  - Rust workspace test
  - target build matrix
  - npm smoke install

## PR 5C. Addon npm Publish Lane Before Root Cutover

### Target Files

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`

### Steps

1. `PR 5A`에서 고정한 addon package name과 target 매핑으로 platform addon npm package tarball을 만든다.
2. CI의 native packaging matrix는 raw native archive smoke에 더해 addon npm tarball을 실제 install/require 해 본다.
3. release workflow는 target build job에서 addon npm tarball을 artifact로 업로드한다.
4. release workflow에 별도 addon publish job을 추가하되 기존 verify gate 뒤에서 addon package를 먼저 publish하고, 그 다음 root package publish가 진행되게 만든다.
5. rerun-safe하게 동작하도록 addon publish도 npm lookup/publish error classifier를 재사용한다.
6. GitHub Release asset에는 raw native archive, addon npm tarball, root npm tarball을 함께 첨부한다.

### Behavior Constraints

- addon publish lane은 `PR 5B` 산출물 위에서만 확장한다. naming source of truth를 새로 만들지 않는다.
- addon npm package는 각 target의 `os`/`cpu` 제한과 `kratos.node` payload만 포함한다.
- addon publish job은 `verify-node`, `verify-rust`, target build 성공 뒤에만 진행된다.
- release workflow에서 root package publish는 addon publish 성공 또는 already-published 판정 뒤에만 진행한다.
- 이 PR도 `package.json`은 수정하지 않는다.

### Validation

- CI dry-run 관점에서 아래 경로가 모두 실행 가능해야 한다.
  - raw native archive smoke
  - addon npm tarball pack
  - addon npm tarball install/require smoke
  - addon publish before root publish

## Done Criteria

- Rust CLI만으로 현재 공개 명령 UX를 재현할 수 있다.
- npm launcher와 native wrapper의 인터페이스가 고정되었다.
- workflow가 Rust artifact release 순서를 표현한다.

## Out Of Scope

- 루트 `package.json` cutover
- JS runtime 삭제
- README/CONTRIBUTING sync
