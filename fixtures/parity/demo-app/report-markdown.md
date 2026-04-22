# Kratos Report

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

## Next Steps

- `kratos report '<REPORT>' --format md`
- `kratos clean '<REPORT>'`

## Broken imports (1)

- src/lib/broken.ts -> `./missing-helper` (static)

## Orphan files (2)

- src/components/DeadWidget.tsx (Component-like module has no inbound references., confidence 0.92)
- src/lib/broken.ts (Module has no inbound references and is not treated as an entrypoint., confidence 0.88)

## Dead exports (3)

- src/components/DeadWidget.tsx -> `DeadWidget`
- src/lib/broken.ts -> `brokenFeature`
- src/lib/math.ts -> `multiply`

## Unused imports (0)

- None

## Route entrypoints (1)

- pages/home.tsx (next-pages-route)

## Deletion candidates (2)

- src/components/DeadWidget.tsx (Component-like module has no inbound references., confidence 0.92, safe true)
- src/lib/broken.ts (Module has no inbound references and is not treated as an entrypoint., confidence 0.88, safe true)
