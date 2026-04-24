# Kratos

[![CI](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml)

한국어 | [English](README.en.md) | [中文](README.zh-CN.md) | [Español](README.es.md) | [日本語](README.ja.md)

죽은 코드를 가차 없이 찾아냅니다.

[라이선스](LICENSE) · [기여 가이드](CONTRIBUTING.md) · [행동 강령](CODE_OF_CONDUCT.md) · [보안](SECURITY.md) · [후원](https://github.com/sponsors/JeremyDev87)

Kratos는 JavaScript/TypeScript 프로젝트에서 사용되지 않는 파일, 끊어진 import, 사용되지 않는 export, 고립된 모듈을 찾아 리포트로 남기는 CLI 도구입니다. 현재 구조는 Rust core/CLI와 npm launcher를 결합하며, npm 패키지 `@jeremyfellaz/kratos`가 플랫폼별 native addon 패키지를 선택적으로 불러 실행합니다.

Kratos는 자동 삭제 도구라기보다 안전한 정리 흐름을 위한 분석 도구입니다. `clean`은 기본적으로 dry-run이며, 실제 삭제는 리포트를 검토한 뒤 `--apply`를 명시했을 때만 수행합니다.

## 핵심 기능

- 사용되지 않는 파일과 고립된 component/module 후보 탐지
- 끊어진 내부 import 탐지
- 사용되지 않는 export와 import 후보 탐지
- Next.js `app/` / `pages/` route entrypoint 휴리스틱
- `tsconfig.json` / `jsconfig.json`의 `baseUrl`, `paths` alias 해석
- `package.json`의 `main`, `module`, `types`, `bin`, `exports` entrypoint 해석
- 저장된 리포트 요약, JSON, Markdown 출력
- 두 리포트 간 finding 변화 비교
- 신뢰도 기준값을 적용한 안전 삭제 후보 미리보기

## 빠른 시작

패키지 사용자 기준 기본 실행 방식은 `npx`입니다.

```bash
npx @jeremyfellaz/kratos scan ./my-app
npx @jeremyfellaz/kratos report ./my-app
npx @jeremyfellaz/kratos report ./my-app --format md
npx @jeremyfellaz/kratos clean ./my-app --min-confidence 0.9
```

리포트를 확인한 뒤 실제 삭제가 필요할 때만 `--apply`를 붙입니다.

```bash
npx @jeremyfellaz/kratos clean ./my-app --apply --min-confidence 0.9
```

두 시점의 리포트를 비교할 수도 있습니다.

```bash
npx @jeremyfellaz/kratos scan ./my-app --output .kratos/before.json
# 코드를 정리하거나 브랜치를 바꾼 뒤
npx @jeremyfellaz/kratos scan ./my-app --output .kratos/after.json
npx @jeremyfellaz/kratos diff ./my-app/.kratos/before.json ./my-app/.kratos/after.json
```

`scan --output`에 상대 경로를 넘기면 스캔 대상 root 기준으로 해석됩니다. 기본 저장 위치는 `<root>/.kratos/latest-report.json`입니다.

## 명령어

### `kratos scan [root] [--output path] [--json]`

프로젝트를 분석하고 report JSON을 저장합니다.

- `root`를 생략하면 현재 작업 디렉터리를 스캔합니다.
- `--output path`는 report 저장 위치를 지정합니다.
- `--json`은 콘솔 요약 대신 전체 JSON report를 stdout으로 출력합니다.
- 기본 출력 경로는 `<root>/.kratos/latest-report.json`입니다.

### `kratos report [report-path-or-root] [--format summary|json|md]`

저장된 report를 사람이 읽기 쉬운 형식 또는 원본 JSON으로 출력합니다.

- `summary`는 기본 콘솔 요약입니다.
- `json`은 저장된 report JSON을 pretty-print합니다.
- `md`는 공유하기 쉬운 Markdown report를 출력합니다.
- 입력이 프로젝트 root이면 `.kratos/latest-report.json`을 자동으로 찾습니다.

### `kratos diff [before-report-path-or-root] [after-report-path-or-root] [--format summary|json|md]`

두 report의 finding 변화를 비교합니다.

- 기본 형식은 `summary`입니다.
- `json`은 introduced/resolved/persisted finding을 machine-readable 형식으로 출력합니다.
- `md`는 리뷰나 이슈에 붙이기 쉬운 Markdown diff를 출력합니다.
- 각 입력은 report 파일 경로 또는 프로젝트 root가 될 수 있습니다.

### `kratos clean [report-path-or-root] [--apply] [--min-confidence value]`

삭제 후보를 preview하거나 실제로 삭제합니다.

- 기본 동작은 dry-run입니다.
- `--apply`를 붙인 경우에만 파일 삭제를 수행합니다.
- `--min-confidence value`는 `0.0`부터 `1.0`까지의 confidence threshold입니다.
- `--min-confidence`를 생략하면 `kratos.config.json`의 `thresholds.cleanMinConfidence`를 사용하고, 설정이 없으면 `0.0`을 사용합니다.

## 결과 예시

`fixtures/demo-app`을 스캔하면 요약 출력은 아래와 같은 형태입니다.

```text
Kratos scan complete.

Root: <root>
Files scanned: 5
Entrypoints: 1
Broken imports: 1
Orphan files: 2
Dead exports: 3
Unused imports: 0
Route entrypoints: 1
Deletion candidates: 2

Saved report: <root>/.kratos/latest-report.json

Broken imports:
- <root>/src/lib/broken.ts -> ./missing-helper

Orphan files:
- <root>/src/components/DeadWidget.tsx
- <root>/src/lib/broken.ts

Dead exports:
- <root>/src/components/DeadWidget.tsx#DeadWidget
- <root>/src/lib/broken.ts#brokenFeature
- <root>/src/lib/math.ts#multiply
```

`clean --min-confidence 0.9`는 신뢰도 기준값을 통과한 삭제 후보와 제외된 후보를 나눠 보여줍니다.

```text
Kratos clean dry run.

Deletion targets: 1
- <root>/src/components/DeadWidget.tsx (confidence 0.92, Component-like module has no inbound references.)

Threshold-skipped targets: 1
- <root>/src/lib/broken.ts (confidence 0.88, Module has no inbound references and is not treated as an entrypoint.)

Re-run with --apply to delete these files.
```

동일한 두 리포트를 비교하면 새로 생기거나 해결된 탐지 결과 없이 유지 중인 개수만 표시됩니다.

```text
Kratos diff complete.

Before: <before-report>
After: <after-report>

Broken imports: introduced 0, resolved 0, persisted 1
Orphan files: introduced 0, resolved 0, persisted 2
Dead exports: introduced 0, resolved 0, persisted 3
Unused imports: introduced 0, resolved 0, persisted 0
Route entrypoints: introduced 0, resolved 0, persisted 1
Deletion candidates: introduced 0, resolved 0, persisted 2

Totals: introduced 0, resolved 0, persisted 9
```

## 리포트 스키마

현재 `scan`은 `schemaVersion: 2` report를 생성합니다.

```json
{
  "schemaVersion": 2,
  "summary": {
    "filesScanned": 5,
    "entrypoints": 1,
    "brokenImports": 1,
    "orphanFiles": 2,
    "deadExports": 3,
    "unusedImports": 0,
    "routeEntrypoints": 1,
    "deletionCandidates": 2
  }
}
```

`findings`에는 `brokenImports`, `orphanFiles`, `deadExports`, `unusedImports`, `routeEntrypoints`, `deletionCandidates`가 들어갑니다. `graph.modules`에는 분석된 모듈 경로, entrypoint 여부, import/export 개수가 기록됩니다.

## 설정

프로젝트 root에 `kratos.config.json`을 둘 수 있습니다. JSONC 스타일 주석과 trailing comma를 허용합니다.

```json
{
  "ignore": ["storybook-static", "generated"],
  "ignorePatterns": ["src/generated/**", "!src/generated/keep.ts"],
  "entry": ["src/bootstrap.ts"],
  "roots": ["src", "app", "pages"],
  "thresholds": {
    "cleanMinConfidence": 0.85
  },
  "suppressions": [
    {
      "kind": "deadExport",
      "file": "src/components/LazyCard.tsx",
      "export": "default",
      "reason": "Loaded dynamically by route metadata."
    },
    {
      "kind": "brokenImport",
      "file": "src/legacy/shim.ts",
      "source": "./generated-shim",
      "reason": "Generated at deploy time."
    }
  ]
}
```

- `ignore`: 기본 ignore 목록에 추가할 디렉터리 이름입니다.
- `ignorePatterns`: `.gitignore` 스타일 경로 패턴입니다. `!`로 예외를 둘 수 있습니다.
- Kratos는 기본 ignore 디렉터리 이후 프로젝트 root의 `.gitignore`를 자동으로 읽고, 그 다음 `ignorePatterns`를 적용해 예외/override를 줄 수 있습니다.
- `entry`: entrypoint로 강제 지정할 프로젝트 root 기준 상대 파일 경로입니다.
- `roots`: 스캔 범위를 제한할 프로젝트 root 기준 상대 디렉터리입니다.
- `thresholds.cleanMinConfidence`: `clean`의 기본 신뢰도 기준값입니다.
- `suppressions`: 의도적으로 무시할 탐지 결과입니다. `kind`는 `brokenImport`, `orphanFile`, `deadExport`, `unusedImport`, `deletionCandidate` 중 하나입니다.

Kratos는 `.kratos/suppressions.json`이 있으면 같은 suppression 형식으로 함께 읽습니다. `file` 값은 프로젝트 root 기준 상대 경로여야 합니다.

## 로컬 개발

필수 조건:

- Node.js 18+
- npm 9+
- Rust stable toolchain

설치:

```bash
npm install
```

권장 검증:

```bash
cargo test --workspace
npm run verify
npm run smoke
```

저장소 checkout에서는 published native addon 패키지가 없을 수 있으므로 `npx @jeremyfellaz/kratos ...`보다 아래 명령이 안전합니다.

```bash
npm run scan -- ./fixtures/demo-app
npm run report -- ./fixtures/demo-app/.kratos/latest-report.json
npm run clean -- ./fixtures/demo-app/.kratos/latest-report.json
cargo run -p kratos-cli -- diff ./fixtures/demo-app ./fixtures/demo-app
```

## 배포 구조

- root npm package는 `@jeremyfellaz/kratos`입니다.
- CLI binary 이름은 `kratos`입니다.
- platform addon package는 macOS arm64/x64, Linux x64/arm64, Windows x64를 대상으로 합니다.
- root package의 `optionalDependencies`는 같은 릴리스 버전의 platform addon package를 가리킵니다.
- raw checkout에서는 native addon이 없을 수 있지만, 릴리스 패키지에서는 launcher가 현재 플랫폼에 맞는 addon을 로드합니다.

## 릴리스 흐름

릴리스는 `vX.Y.Z` 또는 `vX.Y.Z-prerelease.N` 형태의 semver tag를 기준으로 움직입니다.

- `Manual Release Bump` workflow는 대상 버전으로 `package.json`과 platform `optionalDependencies` 버전을 맞추는 version-only PR을 준비합니다.
- tag를 만들기 전에는 같은 commit에서 `cargo test --workspace`, `npm run verify`, native packaging CI가 통과해야 합니다.
- `Release Publish` workflow는 지정된 tag ref를 checkout하고 Node package 검증, Rust workspace test, platform native artifact build를 수행합니다.
- platform addon npm package를 먼저 pack/smoke/publish하고, 마지막에 root package를 publish한 뒤 GitHub Release를 생성하거나 갱신합니다.
- `Release Published Follow-up` workflow는 게시된 릴리스에 대응하는 publish run 성공 여부와 릴리스 asset 존재 여부를 감사합니다. 이 workflow는 publish를 다시 실행하지 않습니다.

release tag push, npm publish, GitHub Release 게시 같은 작업은 유지보수자 확인 후에만 진행합니다.

## 오픈소스

Kratos는 MIT 라이선스로 공개되는 오픈소스 프로젝트입니다.

- 버그 제보와 기능 제안은 GitHub Issues를 사용해 주세요.
- 보안 이슈는 공개 이슈로 올리지 말고 [SECURITY.md](SECURITY.md)의 절차를 따라 주세요.
- 기여 전에는 [CONTRIBUTING.md](CONTRIBUTING.md)와 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)를 확인해 주세요.
- 프로젝트 유지에 도움이 되고 싶다면 [GitHub Sponsors](https://github.com/sponsors/JeremyDev87)를 통해 후원할 수 있습니다.

## 주의

Kratos는 보수적인 정적 분석과 휴리스틱을 함께 사용합니다. 동적 import, framework convention, 생성 파일, runtime-only entrypoint는 프로젝트마다 다르게 해석될 수 있으므로 `--apply` 전에 report와 diff를 검토하는 흐름을 권장합니다.
