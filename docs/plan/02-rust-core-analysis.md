# Rust Core Analysis Plan

## Objective

현재 JS MVP가 제공하는 분석 기능을 `kratos-core` 안에서 Rust typed model로 다시 구현한다. 이 plan family는 입력 해석, Oxc 기반 parser, graph 분석, report v2, clean safety까지를 다루며, CLI나 npm 배포는 포함하지 않는다.

## Conflict Surface

- `crates/kratos-core/src/config.rs`
- `crates/kratos-core/src/jsonc.rs`
- `crates/kratos-core/src/discover.rs`
- `crates/kratos-core/src/resolve.rs`
- `crates/kratos-core/src/parser/mod.rs`
- `crates/kratos-core/src/parser/adapter.rs`
- `crates/kratos-core/src/parser/imports.rs`
- `crates/kratos-core/src/parser/exports.rs`
- `crates/kratos-core/src/parser/unused_imports.rs`
- `crates/kratos-core/src/entrypoints.rs`
- `crates/kratos-core/src/analyze.rs`
- `crates/kratos-core/src/report.rs`
- `crates/kratos-core/src/clean.rs`
- `crates/kratos-core/tests/**`
- `fixtures/parity/**`

## Rules

- `PR 2A`와 `PR 2B`는 파일 충돌이 없도록 분리되어 있으므로 같은 phase에서 병렬 진행할 수 있다.
- `PR 3A`는 `PR 2A`와 `PR 2B`가 모두 머지된 뒤에 시작한다.
- `PR 3B`는 `clean.rs`와 전용 test만 수정한다. `analyze.rs`나 `report.rs`를 같이 만지지 않는다.
- parser 구현은 Oxc를 사용하되, 출력은 JS 구현의 internal shape가 아니라 `model.rs`의 typed record로 맞춘다.
- false positive를 피하기 위한 보수적 동작은 그대로 유지한다. 특히 nested destructuring과 unknown usage는 공격적으로 dead-code로 몰지 않는다.

## PR 2A. Config Discovery And Resolve In Rust

### Target Files

- `crates/kratos-core/src/config.rs`
- `crates/kratos-core/src/jsonc.rs`
- `crates/kratos-core/src/discover.rs`
- `crates/kratos-core/src/resolve.rs`
- `crates/kratos-core/tests/config_and_discovery.rs`

### Steps

1. `config.rs`에서 `package.json`, `tsconfig.json`, `jsconfig.json`, `kratos.config.json`을 읽어 `ProjectConfig`를 구성한다.
2. 지원하는 config key는 현재 JS와 동일하게 고정한다.
   - `ignore`
   - `entry`
   - `roots`
3. `jsonc.rs`는 comments와 trailing comma를 허용하는 loose JSON parser를 제공한다.
4. `discover.rs`는 현재 JS와 동일한 source extension 집합만 수집한다.
   - `.js`
   - `.jsx`
   - `.ts`
   - `.tsx`
   - `.mjs`
   - `.cjs`
   - `.mts`
   - `.cts`
5. `resolve.rs`는 아래 순서로 import target을 해석한다.
   - `node:` builtin
   - relative path
   - root-relative path
   - `paths`
   - `baseUrl`
   - external package
6. directory import는 `index.*` fallback을 지원한다.
7. `package.json` entry는 아래 key만 포함한다.
   - `main`
   - `module`
   - `types`
   - `bin`
   - `exports`

### Behavior Constraints

- 존재하지 않는 configured roots는 에러 대신 skip한다.
- internal path가 없을 때만 broken import 후보가 된다. npm package import는 broken으로 취급하지 않는다.
- path alias는 longest match 우선으로 정렬한다.

### Validation

- `config_and_discovery.rs`에 아래 시나리오를 포함한다.
  - comments/trailing comma가 있는 config 파싱
  - missing root skip
  - `baseUrl` + `paths` alias resolve
  - `package.json` entry collection

## PR 2B. Oxc Parser And Symbol Extraction

### Target Files

- `crates/kratos-core/src/parser/mod.rs`
- `crates/kratos-core/src/parser/adapter.rs`
- `crates/kratos-core/src/parser/imports.rs`
- `crates/kratos-core/src/parser/exports.rs`
- `crates/kratos-core/src/parser/unused_imports.rs`
- `crates/kratos-core/tests/parser_extraction.rs`

### Steps

1. `adapter.rs`에서 파일 확장자 기준으로 JS/TS/JSX/TSX parse mode를 결정한다.
2. `imports.rs`는 아래 패턴을 typed `ImportRecord`로 변환한다.
   - static import
   - side-effect import
   - `export ... from`
   - `export * from`
   - `export * as ns from`
   - dynamic import with string literal
   - CommonJS `require`
   - destructured `require`
3. `exports.rs`는 아래 패턴을 typed `ExportRecord`로 변환한다.
   - `export default`
   - named function/class export
   - variable export
   - named export list
   - `exports.foo =`
   - `module.exports =`
4. `unused_imports.rs`는 identifier usage scan을 AST 기준으로 구현한다.
5. nested destructuring, spread, computed property처럼 판단이 애매한 require binding은 `unknown`으로 처리한다.

### Behavior Constraints

- template interpolation에서 import 식별자를 사용한 경우 unused로 잡지 않는다.
- namespace import는 dead export 계산에서 보수적으로 처리할 수 있도록 `namespace` kind를 유지한다.
- string literal이 아닌 dynamic import는 분석 대상 edge로 만들지 않는다.

### Validation

- `parser_extraction.rs`에 아래 시나리오를 포함한다.
  - namespace re-export
  - template interpolation inside backticks
  - destructured require
  - nested destructuring conservative fallback
  - ternary default inside destructuring

## PR 3A. Graph Analysis And Report V2

### Target Files

- `crates/kratos-core/src/entrypoints.rs`
- `crates/kratos-core/src/analyze.rs`
- `crates/kratos-core/src/report.rs`
- `crates/kratos-core/tests/analyze_demo_app.rs`
- `crates/kratos-core/tests/report_v2.rs`

### Steps

1. `entrypoints.rs`에 현재 JS와 동일한 entrypoint 규칙을 구현한다.
   - user-configured entries
   - package entries
   - `app/**/(page|route|layout|loading|error|not-found).*`
   - `pages/**/*`
   - `src/main.*`, `src/index.*`, `src/bootstrap.*`, `src/cli.*`
   - common tool config files
   - `middleware.*`, `instrumentation.*`
2. `analyze.rs`에서 module graph를 만들고 아래 finding을 계산한다.
   - broken imports
   - orphan files
   - dead exports
   - unused imports
   - route entrypoints
   - deletion candidates
3. orphan classifier의 reason/confidence 문구는 현재 JS 텍스트를 그대로 유지한다.
4. `report.rs`에서 `ReportV2` JSON serialization과 summary/markdown formatter를 구현한다.
5. `report_v2.rs`에서 `fixtures/parity/demo-app/*`를 읽어 normalized parity를 비교한다.
   - summary count는 완전 일치
   - markdown 섹션 구조는 완전 일치
   - JSON은 `schemaVersion: 2` 기준으로 비교

### Behavior Constraints

- report JSON은 반드시 `schemaVersion: 2`를 포함한다.
- `project.root`는 absolute path를 유지한다.
- `project.configPath`는 config가 없으면 `null`로 직렬화한다.
- `graph.modules`는 최소 아래 필드를 가진다.
  - `file`
  - `relativePath`
  - `entrypointKind`
  - `importedByCount`
  - `importCount`
  - `exportCount`

### Validation

- `analyze_demo_app.rs`는 `fixtures/demo-app` 전체 scan 결과를 검증한다.
- `report_v2.rs`는 parity fixture와 normalized comparison을 수행한다.

## PR 3B. Clean Safety And Report V2 Consumption

### Target Files

- `crates/kratos-core/src/clean.rs`
- `crates/kratos-core/tests/clean_safety.rs`

### Steps

1. `clean.rs`에 v2 report를 읽고 deletion candidate를 dry-run/apply 하는 함수들을 구현한다.
2. root boundary check와 symlink escape 차단을 현재 JS와 동일한 의미로 재구현한다.
3. 파일 삭제 후 empty directory cleanup을 구현한다.
4. v1 report가 들어오면 `InvalidReportVersion` 오류를 반환한다.

### Behavior Constraints

- `clean`은 report root 밖의 파일을 삭제하지 않는다.
- symlink parent가 project root 밖으로 나가면 삭제하지 않는다.
- 이미 없는 파일은 에러 없이 skip한다.

### Validation

- `clean_safety.rs`에 아래 시나리오를 포함한다.
  - outside root deletion reject
  - symlink escape reject
  - symlinked project root allow
  - invalid report version reject

## Done Criteria

- `kratos-core`가 config, parsing, graph, report, clean safety를 모두 가진다.
- `demo-app` 기준 parity baseline과 Rust 결과를 비교할 수 있다.
- CLI 없이도 core library 수준에서 MVP 기능이 완성된다.

## Out Of Scope

- `clap` CLI wiring
- npm launcher
- workflow/release 변경
