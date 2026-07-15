# Kratos v1 CLI/report contract evidence

Issue: [#91](https://github.com/JeremyDev87/kratos/issues/91)

This note freezes the consumer-visible v1 CLI/report contract before any version bump. It is intentionally limited to command/report semantics and does not bump versions, create tags, publish packages, or dispatch release workflows.

## Commands in scope

- `kratos scan [root] [--output path] [--no-write] [--json]`
- `kratos report [report-path-or-root] [--format summary|json|md]`
- `kratos diff [before-report-path-or-root] [after-report-path-or-root] [--format summary|json|md]`
- `kratos clean [report-path-or-root] [--apply] [--min-confidence value]`

## Exit-code contract

- Successful command execution returns exit code `0`.
- Unknown commands and invalid explicit format, boolean, threshold, or incompatible option values return exit code `1` with `Kratos 실행 실패: ...` or the Korean command-help error path.
- Compatibility exceptions retained from the JavaScript baseline return `0` when the underlying command succeeds: `scan`/`clean` ignore unknown flags, `report` ignores surplus positionals, and bare or empty `report --format` falls back to `summary`.
- `clean` without `--apply` is a preview/dry-run and returns `0` when the report is valid.
- `clean --apply` returns `0` after completing its delete/skip plan, including a successful no-op, and returns `1` for invalid input or an unhandled filesystem error.

## Report JSON contract

The v1 writer emits schema version `2`. Stable writer key names are centralized in the private `report_contract` module and used by the serializer; independent literal expectations in the contract test freeze the emitted shape.

Required top-level keys:

- `schemaVersion`
- `generatedAt`
- `project`
- `summary`
- `findings`
- `graph`

Required nested keys:

- `project`: `root`, `configPath`
- `summary`: `filesScanned`, `entrypoints`, `brokenImports`, `orphanFiles`, `deadExports`, `unusedImports`, `routeEntrypoints`, `deletionCandidates`
- optional `summary`: `suppressedFindings` only when non-zero
- `findings`: `brokenImports`, `orphanFiles`, `deadExports`, `unusedImports`, `routeEntrypoints`, `deletionCandidates`
- `graph`: `modules`
- `graph.modules[]`: `file`, `relativePath`, `entrypointKind`, `importedByCount`, `importCount`, `exportCount`

Finding item shapes:

- `brokenImports[]`: `file`, `source`, `kind`
- `orphanFiles[]`: `file`, `kind`, `reason`, `confidence`
- `deadExports[]`: `file`, `exportName`, `exportKind`, `reason`, `confidence`, `importedByCount`, `usedExportNames`, `hasNamespaceOrUnknownUsage`
- `unusedImports[]`: `file`, `source`, `local`, `imported`
- `routeEntrypoints[]`: `file`, `kind`
- `deletionCandidates[]`: `file`, `reason`, `confidence`, `safe`

The writer-key contract test uses independent literal expectations for every container, finding item, and graph module key above. It also fixes representative item values/types; existing parity and round-trip tests cover broader serialized values and reader compatibility.

## Format contract

- `scan --json` writes JSON only to stdout and does not include human guidance text.
- `scan --no-write` does not create `.kratos/latest-report.json`.
- `report --format summary` emits the Korean human summary.
- `report --format json` pretty-prints the parsed input JSON shape.
- `report --format md` emits Markdown headed by `# Kratos 리포트`.
- `diff --format summary` emits the Korean diff summary headed by `Kratos diff 완료.`.
- `diff --format json` emits machine-readable diff JSON.
- `diff --format md` emits Markdown headed by `# Kratos Diff 결과`.
- `clean` emits a Korean dry-run preview unless `--apply` is supplied.

## Clean execution contract

- A report with no deletion candidates is a successful no-op (`0`) before threshold configuration is loaded.
- `--min-confidence` overrides `thresholds.cleanMinConfidence`; when neither is provided, the threshold is `0.0`. Candidates below the effective threshold are skipped.
- Preview excludes candidates whose normalized path or real parent escapes the report root. Root-contained missing or unreadable candidates remain visible with status/marker evidence instead of being silently omitted.
- Apply skips root-escaping, missing, and below-threshold candidates. `clean --apply` reports deleted and skipped counts; non-`NotFound` deletion errors return `1`, and cleanup of now-empty parent directories is best-effort within the report root.
- The required `safe` field is descriptive metadata in the current v1 implementation; apply is gated by deletion-candidate membership, confidence, filesystem existence, and root containment. Consumers must not treat `safe` alone as deletion authorization. Fail-closed hardening remains follow-up work for issue #92.

## Evidence added in this PR

- `crates/kratos-core/tests/report_v2.rs::report_v2_writer_keys_match_the_frozen_v1_contract` verifies emitted writer keys against independent literal key lists and representative value/type assertions.
- `crates/kratos-cli/tests/cli_smoke.rs::scan_report_and_clean_work_for_demo_fixture` now includes diff summary/json smoke evidence alongside scan/report/clean evidence.
