# Kratos Report

> 6 actionable findings: 1 broken import, 2 cleanup candidates, 3 dead exports.

- Generated: <GENERATED_AT>
- Root: <ROOT>
- Report: <REPORT>

## Summary

- Files scanned: 5
- Entrypoints: 1
- Broken imports: 1
- Orphan files: 2
- Dead exports: 3
- Unused imports: 0
- Route entrypoints: 1
- Deletion candidates: 2

## Impact

- 6 actionable findings: 1 broken import, 2 cleanup candidates, 3 dead exports.
- Best next move: Fix broken imports before deleting files.
- Preview cleanup: `kratos clean <REPORT>`
- Refresh markdown: `kratos report <REPORT> --format md`

## Broken imports

- src/lib/broken.ts -> `./missing-helper`

## Orphan files

- src/components/DeadWidget.tsx (Component-like module has no inbound references.)
- src/lib/broken.ts (Module has no inbound references and is not treated as an entrypoint.)

## Dead exports

- src/components/DeadWidget.tsx -> `DeadWidget`
- src/lib/broken.ts -> `brokenFeature`
- src/lib/math.ts -> `multiply`

## Unused imports

- None

## Route entrypoints

- pages/home.tsx (next-pages-route)

## Deletion candidates

- src/components/DeadWidget.tsx (Component-like module has no inbound references., confidence 0.92)
- src/lib/broken.ts (Module has no inbound references and is not treated as an entrypoint., confidence 0.88)
