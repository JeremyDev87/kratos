# Parity Fixtures

이 디렉터리는 현재 CLI 구현을 기준으로 고정한 parity baseline을 담습니다.
향후 동작 변경은 이 산출물과 비교하면서 검증합니다.

규칙:

- 사람이 fixture 내용을 직접 수정하지 않습니다.
- fixture는 `node ./scripts/capture-parity-fixtures.mjs`로만 갱신합니다.
- 절대 경로는 `<ROOT>`와 `<REPORT>` placeholder로 정규화됩니다.
- 생성 시각은 `<GENERATED_AT>` placeholder로 고정됩니다.

재생성 명령:

```bash
node ./scripts/capture-parity-fixtures.mjs
```
