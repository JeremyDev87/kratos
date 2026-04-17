# CLI Help Modularization Plan

## Objective

CLI 루트 help 텍스트를 `src/cli.js`의 고정 문자열에서 분리해, 각 command 파일이 자기 usage와 summary를 직접 소유하도록 바꾼다. 이 작업의 목적은 이후 `scan`, `report`, `clean` 기능 확장 PR이 help 문구 변경 때문에 `src/cli.js`와 충돌하지 않게 만드는 것이며, 실제 command 동작이나 출력 포맷 자체를 바꾸는 것은 아니다.

## Conflict Surface

- `src/cli.js`
- `src/commands/scan.js`
- `src/commands/report.js`
- `src/commands/clean.js`
- `src/lib/help.js`
- `test/cli-help.test.js`

## Rules

- 기존 root help 진입점인 `kratos`, `kratos --help`, `kratos -h`, `kratos help`는 계속 동작해야 한다.
- command 실행 동작은 바꾸지 않고, help metadata 소유권만 command 파일 쪽으로 이동한다.
- `scan`, `report`, `clean`의 usage 문구 변경은 이후 해당 command 파일만 수정하면 되도록 경계를 만든다.
- unknown command 실패 시에는 non-zero 종료와 함께 root help를 계속 보여준다.
- 이 PR에서는 새 command를 추가하지 않는다.

## Slice 1. Command Metadata Extraction

### Target Files

- `src/cli.js`
- `src/commands/scan.js`
- `src/commands/report.js`
- `src/commands/clean.js`
- `new helper file: src/lib/help.js`
- `new helper spec: test/cli-help.test.js`

### Steps

1. `src/lib/help.js`에 root help와 command help를 포맷하는 공용 함수를 만든다.
2. `src/commands/scan.js`, `src/commands/report.js`, `src/commands/clean.js`가 각각 자기 command metadata를 export 하게 바꾼다.
3. `src/cli.js`는 command metadata 목록을 기반으로 dispatch와 help 출력을 수행하게 바꾼다.
4. `kratos <command> --help`가 해당 command metadata에서 직접 나온 usage를 보여주게 만든다.
5. unknown command 시 stderr에 root help를 함께 출력하는 회귀 테스트를 추가한다.

## Done Criteria

- [ ] root help가 `src/lib/help.js`를 통해 생성된다.
- [ ] `scan`, `report`, `clean` help metadata가 각 command 파일에 위치한다.
- [ ] 이후 command 옵션 문구 변경이 `src/cli.js`를 수정하지 않고도 가능하다.
- [ ] CLI help 회귀 테스트가 추가되고 통과한다.

## Out Of Scope

- `diff` 같은 신규 command 추가
- `scan`, `report`, `clean`의 실제 기능 변경
- README 문서 갱신
- command별 상세 examples, 색상 출력, interactive help UI
