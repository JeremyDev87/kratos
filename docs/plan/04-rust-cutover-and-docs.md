# Rust Cutover And Docs Plan

## Objective

Rust 경로가 준비된 뒤, npm package의 실제 실행 엔트리포인트를 Rust launcher로 바꾸고 기존 JS 런타임을 제거한다. 마지막으로 사용자 문서와 릴리스 절차를 새 구조에 맞춰 동기화하고 alpha 배포 준비를 마친다.

## Conflict Surface

- `package.json`
- `.gitignore`
- `scripts/release-plan.mjs`
- `scripts/classify-npm-lookup-error.mjs`
- `scripts/classify-npm-publish-error.mjs`
- `scripts/lib/release.mjs`
- `fixtures/release-errors/npm-lookup-not-found.txt`
- `fixtures/release-errors/npm-publish-already-published.txt`
- `src/cli.js`
- `src/commands/scan.js`
- `src/commands/report.js`
- `src/commands/clean.js`
- `src/lib/*.js`
- `test/cli-help.test.js`
- `test/kratos.test.js`
- `test/package-smoke.test.js`
- `README.md`
- `README.en.md`
- `README.es.md`
- `README.ja.md`
- `README.zh-CN.md`
- `CONTRIBUTING.md`

## Rules

- `PR 6A` 전에는 JS 런타임 source tree를 삭제하지 않는다.
- `PR 6A`는 문서 파일을 수정하지 않는다. 문서는 `PR 7A`만 수정한다.
- `package.json`과 `.gitignore`는 `PR 6A`에서 수정하고, `package.json`은 stable promotion용 `Release Prep 8A`에서 한 번 더 수정한다.
- `PR 6A`는 `src/lib/release.js`를 삭제하기 전에 release helper script들의 import 경로를 `src/` 밖으로 옮겨야 한다.
- final verification은 일반 병렬 PR이 아니라 별도 gate로 유지한다.

## PR 6A. Root Package Cutover And JS Runtime Removal

### Target Files

- `package.json`
- `.gitignore`
- `bin/kratos.js`
- `scripts/release-plan.mjs`
- `scripts/classify-npm-lookup-error.mjs`
- `scripts/classify-npm-publish-error.mjs`
- add `scripts/lib/release.mjs`
- add `fixtures/release-errors/npm-lookup-not-found.txt`
- add `fixtures/release-errors/npm-publish-already-published.txt`
- delete `src/cli.js`
- delete `src/commands/scan.js`
- delete `src/commands/report.js`
- delete `src/commands/clean.js`
- delete `src/lib/args.js`
- delete `src/lib/analyze.js`
- delete `src/lib/config.js`
- delete `src/lib/constants.js`
- delete `src/lib/discover.js`
- delete `src/lib/fs.js`
- delete `src/lib/help.js`
- delete `src/lib/jsonc.js`
- delete `src/lib/parser.js`
- delete `src/lib/report.js`
- delete `src/lib/release.js`
- delete `src/lib/resolve.js`
- delete `test/cli-help.test.js`
- delete `test/kratos.test.js`
- add `test/package-smoke.test.js`

### Steps

1. `package.json`의 `bin.kratos`를 `./bin/kratos.js`로 바꾸고 version을 첫 Rust alpha용 `0.2.0-alpha.1`로 맞춘다.
2. `files` 목록에서 `src`를 제거하고, Rust/NAPI 배포에 필요한 경로만 남긴다.
3. `scripts.scan`, `scripts.report`, `scripts.clean`, `scripts.smoke`, `scripts.test`, `scripts.verify`를 새 실행 구조 기준으로 재작성한다.
4. `scripts/release-plan.mjs`, `scripts/classify-npm-lookup-error.mjs`, `scripts/classify-npm-publish-error.mjs`가 더 이상 `src/lib/release.js`를 import하지 않도록 `scripts/lib/release.mjs` 또는 동등한 non-`src/` helper로 옮긴다.
5. classifier script smoke에 사용할 고정 stderr fixture 두 개를 추가한다.
   - `fixtures/release-errors/npm-lookup-not-found.txt`
   - `fixtures/release-errors/npm-publish-already-published.txt`
6. `.gitignore`에 Rust build output과 release 산출물 패턴을 추가한다.
   - `target/`
   - platform artifact staging dir가 있으면 그 경로
7. 기존 JS runtime source tree와 JS-only tests를 삭제한다.
8. `test/package-smoke.test.js`는 root package install/launcher smoke만 검증한다.

### Behavior Constraints

- 이 PR은 문서를 갱신하지 않는다. 코드/패키지 cutover만 다룬다.
- npm tarball에 `src/` 런타임이 포함되면 실패로 간주한다.
- `bin/kratos.js`는 이미 `PR 5A`에서 만든 로직을 그대로 사용하고, 여기서는 package wiring만 바꾼다.
- release helper script는 `src/lib/release.js` 삭제 후에도 독립적으로 실행 가능해야 한다.

### Validation

- `npm pack --dry-run --cache ./.npm-cache` 결과에서 `src/`가 빠졌는지 확인한다.
- `node ./scripts/release-plan.mjs v0.2.0-alpha.1`가 import 오류 없이 동작하는지 확인한다.
- `node ./scripts/classify-npm-lookup-error.mjs ./fixtures/release-errors/npm-lookup-not-found.txt`가 `not-found`를 출력하는지 확인한다.
- `node ./scripts/classify-npm-publish-error.mjs ./fixtures/release-errors/npm-publish-already-published.txt`가 `already-published`를 출력하는지 확인한다.
- root package 기준 `npx @jeremyfellaz/kratos scan ./fixtures/demo-app` smoke를 수행한다.

## PR 7A. Docs Sync And Alpha Release Readiness

### Target Files

- `README.md`
- `README.en.md`
- `README.es.md`
- `README.ja.md`
- `README.zh-CN.md`
- `CONTRIBUTING.md`

### Steps

1. Quick start를 Rust-backed npm launcher 기준으로 갱신한다.
2. 로컬 개발 섹션에 Rust toolchain prerequisite를 추가한다.
3. report schema 변경을 문서화한다.
   - `schemaVersion: 2`
   - `clean`은 v2 report만 지원
4. release 절차를 Rust artifact + root package publish 흐름에 맞춰 갱신한다.
5. alpha rollout 순서를 명시한다.
   - `0.2.0-alpha.1`
   - alpha 검증
6. stable `0.2.0` 승격은 alpha 검증 후 별도의 version-only release-prep commit에서 `package.json`을 갱신한 뒤 tag를 만드는 흐름으로 문서화한다.

### Behavior Constraints

- 문서는 실제 landed behavior만 설명한다.
- 다국어 README는 같은 정보 구조를 유지한다. 한 언어만 최신 상태가 되면 안 된다.

### Validation

- 모든 README의 quick start, local development, release 섹션이 같은 구조인지 확인한다.
- `CONTRIBUTING.md`가 새 build/test 흐름과 모순되지 않는지 확인한다.

## Verification Gate 7B. Full Multi-Platform Verification

### Checks

- `cargo test --workspace`
- Node wrapper smoke
- `npx @jeremyfellaz/kratos scan ./fixtures/demo-app`
- `npx @jeremyfellaz/kratos report ./fixtures/demo-app/.kratos/latest-report.json`
- `npx @jeremyfellaz/kratos clean ./fixtures/demo-app/.kratos/latest-report.json`
- `npm pack --dry-run --cache ./.npm-cache`
- Linux/macOS/Windows target artifact build
- one macOS arm64 smoke

### Exit Criteria

- Rust CLI, launcher, workflows, package contents, docs가 모두 일치한다.
- alpha release 후보가 만들어질 수 있다.

## Release Prep 8A. Version-Only Stable Promotion

### Target Files

- `package.json`

### Steps

1. alpha 검증이 끝난 뒤 `package.json` version을 `0.2.0-alpha.1`에서 `0.2.0`으로 올린다.
2. 이 단계에서는 런타임 코드나 문서를 수정하지 않는다. version만 바꾼다.
3. stable tag는 이 version-only commit이 merge된 뒤에만 만든다.

### Behavior Constraints

- 이 단계는 `package.json` serial lane의 예외로, stable release promotion만 다룬다.
- alpha에서 stable로 갈 때 feature나 refactor를 섞지 않는다.

### Validation

- `node -p "require('./package.json').version"`이 `0.2.0`을 출력하는지 확인한다.
- `node ./scripts/release-plan.mjs v0.2.0`가 import 오류 없이 동작하는지 확인한다.

## Done Criteria

- root npm package가 실제로 Rust 런타임을 사용한다.
- 기존 JS runtime source tree가 repo 런타임 경로에서 제거됐다.
- 사용자 문서가 새 구조와 일치한다.

## Out Of Scope

- stable release 이후 기능 확장
- monorepo/watch/diff 등 차기 기능 추가
