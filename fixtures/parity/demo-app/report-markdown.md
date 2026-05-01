# Kratos 리포트

> 조치할 항목 6개: 깨진 import 1개, 정리 후보 2개, 사용되지 않는 export 3개.

- 생성 시각: <GENERATED_AT>
- 루트: <ROOT>
- 리포트: <REPORT>

## 요약

- 스캔한 파일: 5
- 진입점: 1
- 깨진 import: 1
- 고아 파일: 2
- 사용되지 않는 export: 3
- 사용되지 않는 import: 0
- 라우트 진입점: 1
- 삭제 후보: 2

## 영향

- 조치할 항목 6개: 깨진 import 1개, 정리 후보 2개, 사용되지 않는 export 3개.
- 다음 권장 작업: 파일을 삭제하기 전에 깨진 import를 먼저 수정하세요.
- 정리 미리보기: `kratos clean <REPORT>`
- Markdown 갱신: `kratos report <REPORT> --format md`

## 깨진 import

- src/lib/broken.ts -> `./missing-helper`

## 고아 파일

- src/components/DeadWidget.tsx (컴포넌트로 보이는 모듈에 참조가 없습니다.)
- src/lib/broken.ts (모듈에 참조가 없고 진입점으로 취급되지 않습니다.)

## 사용되지 않는 export

- src/components/DeadWidget.tsx -> `DeadWidget`
- src/lib/broken.ts -> `brokenFeature`
- src/lib/math.ts -> `multiply`

## 사용되지 않는 import

- 없음

## 라우트 진입점

- pages/home.tsx (next-pages-route)

## 삭제 후보

- src/components/DeadWidget.tsx (컴포넌트로 보이는 모듈에 참조가 없습니다., 신뢰도 0.92)
- src/lib/broken.ts (모듈에 참조가 없고 진입점으로 취급되지 않습니다., 신뢰도 0.88)
