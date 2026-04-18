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

## Broken imports

- <ROOT>/src/lib/broken.ts -> `./missing-helper`

## Orphan files

- <ROOT>/src/components/DeadWidget.tsx (Component-like module has no inbound references.)
- <ROOT>/src/lib/broken.ts (Module has no inbound references and is not treated as an entrypoint.)

## Dead exports

- <ROOT>/src/components/DeadWidget.tsx -> `DeadWidget`
- <ROOT>/src/lib/broken.ts -> `brokenFeature`
- <ROOT>/src/lib/math.ts -> `multiply`

## Unused imports

- None

## Route entrypoints

- <ROOT>/pages/home.tsx (next-pages-route)

## Deletion candidates

- <ROOT>/src/components/DeadWidget.tsx (Component-like module has no inbound references., confidence 0.92)
- <ROOT>/src/lib/broken.ts (Module has no inbound references and is not treated as an entrypoint., confidence 0.88)
