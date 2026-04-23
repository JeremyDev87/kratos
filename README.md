# Kratos

[![CI](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml)

한국어 | [English](README.en.md) | [中文](README.zh-CN.md) | [Español](README.es.md) | [日本語](README.ja.md)

Destroy dead code ruthlessly.

[License](LICENSE) · [Contributing](CONTRIBUTING.md) · [Code of Conduct](CODE_OF_CONDUCT.md) · [Security](SECURITY.md) · [Sponsor](https://github.com/sponsors/JeremyDev87)

Kratos는 프로젝트 안에 숨어 있는 죽은 코드, 사용되지 않는 파일, 끊어진 import, orphan module을 찾아 제거 후보를 제안하는 CLI 도구입니다. 현재 배포 구조는 Rust core/CLI와 npm launcher를 결합한 형태이며, 보수적인 분석과 안전한 삭제 후보 제안에 초점을 둡니다.

## 핵심 역할

- 사용되지 않는 파일 탐지
- dead export 탐지
- broken import 탐지
- orphan module / orphan component 탐지
- 안전 삭제 후보 제안
- 코드베이스 슬림화 리포트 제공

## 빠른 시작

패키지 사용자 기준 기본 흐름은 `npx`입니다.

```bash
npx @jeremyfellaz/kratos scan ./your-project
npx @jeremyfellaz/kratos report ./your-project/.kratos/latest-report.json
npx @jeremyfellaz/kratos clean ./your-project/.kratos/latest-report.json
```

- `scan`은 기본적으로 `.kratos/latest-report.json`을 생성합니다.
- `clean`은 기본이 dry-run이며, 실제 삭제는 `--apply`를 붙였을 때만 수행합니다.

## 로컬 개발

저장소 체크아웃에서는 Rust CLI와 npm script를 기준으로 개발합니다.

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

레포 안에서 직접 동작을 확인할 때는 아래 명령을 사용합니다.

```bash
npm run scan -- ./fixtures/demo-app
npm run report -- ./fixtures/demo-app/.kratos/latest-report.json
npm run clean -- ./fixtures/demo-app/.kratos/latest-report.json
```

체크아웃 상태에서는 published native addon 패키지가 없을 수 있으므로 `npx @jeremyfellaz/kratos ...` 대신 위 명령이나 `cargo run -p kratos-cli -- ...`를 사용하는 것이 안전합니다.

## 명령어

### `kratos scan [root]`

프로젝트를 스캔하고 최신 리포트를 저장합니다.

- 기본 출력 경로: `<root>/.kratos/latest-report.json`
- `--output <path>`: 리포트 저장 위치를 직접 지정
- `--json`: 콘솔 요약 대신 전체 JSON 리포트를 stdout으로 출력

### `kratos report [report-path-or-root]`

저장된 리포트를 요약, JSON, Markdown 형식으로 출력합니다.

- `--format summary`: 기본 요약 출력
- `--format json`: 원본 JSON 출력
- `--format md`: Markdown 리포트 출력
- 리포트 경로 대신 프로젝트 root를 넘기면 최신 리포트를 자동으로 찾습니다

### `kratos clean [report-path-or-root]`

삭제 후보를 보여주거나 실제로 삭제합니다.

- 기본 동작은 dry-run입니다
- `--apply`: 실제 삭제 수행
- 리포트 경로 대신 프로젝트 root를 넘기면 최신 리포트를 자동으로 찾습니다

## 리포트 스키마

현재 Rust scan 출력은 `schemaVersion: 2`를 기록합니다.

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

- 기본 저장 위치는 `.kratos/latest-report.json`입니다
- `report`와 `clean`은 저장된 리포트 경로 또는 프로젝트 root를 입력으로 받을 수 있습니다
- Markdown 출력에는 broken import, orphan file, dead export, route entrypoint, deletion candidate가 함께 요약됩니다

## 현재 탐지 범위

- Rust analyzer + Oxc 기반 JS/TS import/export 파싱
- relative import / require / dynamic import
- `tsconfig.json` / `jsconfig.json`의 `baseUrl`, `paths`
- Next.js `app/` / `pages/` route entrypoint 휴리스틱
- `package.json`의 `main`, `module`, `bin`, `exports` 진입점
- orphan file / orphan component 후보
- dead export 후보
- unused import 후보
- broken internal import

## 설정

선택적으로 `kratos.config.json`을 둘 수 있습니다.

```json
{
  "ignore": ["storybook-static", "generated"],
  "entry": ["src/bootstrap.ts"],
  "roots": ["src", "app", "pages"]
}
```

- `ignore`: 추가로 무시할 디렉터리 이름
- `entry`: 엔트리포인트로 강제 지정할 파일 경로
- `roots`: 스캔 루트를 제한할 폴더 경로

## 추천 대상

- 오래된 React / Next.js 프로젝트
- 기능 출시가 많아 코드가 누적된 팀
- 리팩터링 타이밍을 찾는 팀

## 릴리스 흐름

Kratos는 `v0.2.0-alpha.1`, `v0.2.0` 같은 시맨틱 버전 태그로 릴리스를 구동합니다.

Alpha 후보 준비:

- root package version은 `0.2.0-alpha.1`을 유지합니다
- 태그를 만들기 전 `cargo test --workspace`, `npm run verify`, `npm run smoke`, fixture 기반 `scan/report/clean` smoke를 완료합니다
- alpha 태그 생성과 push는 유지보수자 확인 후 진행합니다

`Release Publish` workflow는 태그 push 또는 기존 태그를 지정한 수동 실행에서 아래를 수행합니다.

- release metadata를 해석하고 prerelease면 npm dist-tag를 `next`로 잡습니다
- Node package verification과 Rust workspace test를 분리해서 실행합니다
- macOS arm64/x64, Linux x64/arm64, Windows x64용 native artifact를 빌드합니다
- platform addon npm package를 pack/smoke한 뒤 먼저 publish합니다
- 마지막에 root `kratos` package를 publish하고 GitHub Release를 생성합니다

`Release Published Follow-up` workflow는 GitHub Release가 published 되었을 때 아래를 수행합니다.

- 해당 release tag에 대응하는 `Release Publish` run이 실제로 있었는지 확인합니다
- 가장 최근 publish run이 success인지, release asset이 비어 있지 않은지 점검합니다
- 이 workflow는 publish를 다시 하지 않고, release 게시 이후 상태만 감사(audit)합니다

Stable 승격:

- alpha 검증이 끝난 뒤 `package.json` version을 `0.2.0-alpha.1`에서 `0.2.0`으로 올리는 version-only release-prep commit을 따로 만듭니다
- stable tag `v0.2.0`은 그 다음 단계에서 별도로 생성하며 npm `latest`로 배포됩니다

권장 배포 방식은 npm Trusted Publishing(OIDC)입니다. 필요하면 저장소 `NPM_TOKEN` secret fallback도 사용할 수 있습니다.

## 오픈소스

Kratos는 MIT 라이선스로 공개되는 오픈소스 프로젝트입니다.

- 버그 제보와 기능 제안은 GitHub Issues를 사용해 주세요
- 보안 이슈는 공개 이슈로 올리지 말고 [SECURITY.md](SECURITY.md)의 절차를 따라 주세요
- 기여 전에는 [CONTRIBUTING.md](CONTRIBUTING.md)와 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)를 확인해 주세요
- 프로젝트 유지에 도움이 되고 싶다면 [GitHub Sponsors](https://github.com/sponsors/JeremyDev87)를 통해 후원할 수 있습니다

## 주의

현재 alpha는 Rust core와 Oxc 기반 파싱을 사용하지만, entrypoint 판정과 안전 삭제 후보 선정은 여전히 보수적인 휴리스틱을 포함합니다. `--apply` 전에 반드시 리포트를 검토하는 흐름을 권장합니다.
