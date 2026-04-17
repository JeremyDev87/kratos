# Kratos

[![CI](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml)

한국어 | [English](README.en.md) | [中文](README.zh-CN.md) | [Español](README.es.md) | [日本語](README.ja.md)

Destroy dead code ruthlessly.

[License](LICENSE) · [Contributing](CONTRIBUTING.md) · [Code of Conduct](CODE_OF_CONDUCT.md) · [Security](SECURITY.md) · [Sponsor](https://github.com/sponsors/JeremyDev87)

Kratos는 프로젝트 안에 숨어 있는 죽은 코드, 사용되지 않는 파일, 끊어진 import, orphan module을 찾아 제거 후보를 제안하는 CLI 도구입니다. 레거시가 쌓일수록 코드베이스는 무거워지고, 유지보수 비용은 증가합니다. Kratos는 불필요한 흔적을 드러내고 코드베이스를 다시 날렵하게 만드는 데 집중합니다.

## 핵심 역할

- 사용되지 않는 파일 탐지
- dead export 탐지
- broken import 탐지
- orphan module / orphan component 탐지
- 안전 삭제 후보 제안
- 코드베이스 슬림화 리포트 제공

## 빠른 시작

```bash
npm install
npx kratos scan
npx kratos report
npx kratos clean
```

로컬 개발 중에는 아래처럼 실행할 수 있습니다.

```bash
node ./src/cli.js scan
node ./src/cli.js report
node ./src/cli.js clean
```

## 명령어

### `kratos scan [root]`

프로젝트를 스캔하고 `.kratos/latest-report.json`에 분석 결과를 저장합니다.

옵션:

- `--output <path>`: 리포트 저장 위치를 직접 지정
- `--json`: 콘솔 요약 대신 JSON 전체를 출력

### `kratos report [report-path-or-root]`

가장 최근 리포트를 읽어 사람이 보기 쉬운 형태로 출력합니다.

옵션:

- `--format summary`: 기본 요약 출력
- `--format json`: 원본 JSON 출력
- `--format md`: Markdown 리포트 출력

### `kratos clean [report-path-or-root]`

삭제 후보를 보여주거나 실제로 삭제합니다.

옵션:

- `--apply`: 실제 삭제 수행

기본 동작은 dry-run입니다. `--apply` 없이는 아무 파일도 삭제하지 않습니다.

## 현재 MVP가 탐지하는 것

- JS / JSX / TS / TSX / MJS / CJS 파일 그래프
- relative import / require / dynamic import
- `tsconfig.json` / `jsconfig.json`의 `baseUrl`, `paths`
- Next.js `app/` / `pages/` 엔트리 파일 휴리스틱
- package.json의 `main`, `module`, `bin`, `exports` 진입점
- orphan file / orphan component 후보
- dead export 후보
- unused import 후보
- broken internal import

## 리포트 예시

```bash
$ node ./src/cli.js scan ./fixtures/demo-app
Kratos scan complete.

Root: /.../fixtures/demo-app
Files scanned: 5
Entrypoints: 1
Broken imports: 1
Orphan files: 2
Dead exports: 3
Unused imports: 0
Deletion candidates: 2

Saved report: /.../fixtures/demo-app/.kratos/latest-report.json
```

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

## 릴리스

Kratos는 `v0.1.0` 같은 시맨틱 버전 태그 기준으로 릴리스합니다.

```bash
npm version 0.1.0 --no-git-tag-version
git add package*.json
git commit -m "chore: release v0.1.0"
git tag v0.1.0
git push origin HEAD
git push origin v0.1.0
```

태그가 푸시되면 [Release workflow](.github/workflows/release.yml)가 아래를 수행합니다.

- `npm run verify` 실행
- npm 배포용 tarball 생성
- stable 릴리스는 npm `latest`, prerelease는 npm `next`로 publish 수행
- GitHub Release 생성 및 tarball 첨부

권장 방식은 npm Trusted Publishing(OIDC)입니다. 아직 설정하지 않았다면 저장소 `NPM_TOKEN` secret으로도 fallback 배포가 가능합니다.

## 오픈소스

Kratos는 MIT 라이선스로 공개되는 오픈소스 프로젝트입니다.

- 버그 제보와 기능 제안은 GitHub Issues를 사용해 주세요.
- 보안 이슈는 공개 이슈로 올리지 말고 [SECURITY.md](SECURITY.md)의 절차를 따라 주세요.
- 기여 전에는 [CONTRIBUTING.md](CONTRIBUTING.md)와 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)를 확인해 주세요.
- 프로젝트 유지에 도움이 되고 싶다면 [GitHub Sponsors](https://github.com/sponsors/JeremyDev87)를 통해 후원할 수 있습니다.

## 주의

이 버전은 AST 기반이 아닌 휴리스틱 기반 MVP입니다. 큰 프로젝트에서도 빠르게 훑어보는 데 초점을 맞췄고, 삭제 전에는 반드시 리포트를 검토하는 흐름을 권장합니다.
