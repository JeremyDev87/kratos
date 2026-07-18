# Kratos v1 CLI/report contract evidence

Issues: [#91](https://github.com/JeremyDev87/kratos/issues/91) contract baseline; [#92](https://github.com/JeremyDev87/kratos/issues/92) clean-safety migration; [#93](https://github.com/JeremyDev87/kratos/issues/93) packed consumer artifact smoke; [#94](https://github.com/JeremyDev87/kratos/issues/94) release provenance evidence

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
- `clean --apply` returns `0` after completing a quarantine/skip plan, including a successful no-op. It returns `1` for invalid input or any per-file filesystem failure after reporting quarantined/skipped/failed counts and the failed path.

## Report JSON contract

The frozen v1 baseline emitted schema version `2`. Issue #92 advances the current writer to schema version `3` because apply-time deletion safety now requires persisted content fingerprints. Stable writer key names remain centralized in the private `report_contract` module and independent literal expectations freeze each emitted shape.

Required top-level keys:

- `schemaVersion`
- `generatedAt`
- `project`
- `summary`
- `findings`
- `cleanSafety`
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

Schema-v3 clean-safety shape:

- `cleanSafety`: `fingerprintAlgorithm`, `candidates`
- `cleanSafety.candidates[]`: `file`, `fingerprint`, `identity`, `parentIdentity`
- `fingerprintAlgorithm` is currently `sha256`; `fingerprint` is a lowercase 64-character digest of the same bytes used by analysis, `identity` is the platform stable-file identity captured from that opened regular file, and `parentIdentity` is the stable identity of the candidate's parent directory captured alongside it. Any evidence field is `null` when it cannot be collected, making the candidate non-deletable.
- Schema-v2 and legacy reports remain readable for summary/Markdown/diff compatibility, but their deletion candidates do not have fingerprint evidence and therefore fail closed during `clean`.

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

## Diff finding identity contract

- Diff artifacts expose `identityVersion: 1`; this versions finding identity independently and does not change the report writer's schema version `3`.
- Every JSON finding has a full lowercase `kratos:v1:<64-hex-sha256>` `id`. Markdown prints the same full ID beside every finding and declares the identity version near the artifact paths.
- The canonical digest input is length-delimited and contains only the identity version, finding kind, normalized root-relative path, and kind-specific semantic locator. Timestamps, reason copy, confidence, `safe`, `cleanSafety`, and other presentation/evidence metadata are excluded.
- Kind-specific semantic locators are unresolved import source/kind, export name, imported source/local/imported tuple, and route-entrypoint kind. Orphan files and deletion candidates use their normalized file path without mutable reason/classification/safety metadata.
- Finding groups and duplicate instances are sorted deterministically before introduced/resolved/persisted output is materialized. Duplicate count changes retain the shared instances and classify only the excess instances as introduced or resolved.
- The existing `ReportDiff` struct literal and legacy core formatters remain source/output compatible. The CLI uses the additive `ReportDiffWithIdentity` and `*_with_identity` APIs to emit the versioned evidence contract without changing legacy library output implicitly.

## Clean execution contract

- A report with no deletion candidates is a successful no-op (`0`) before threshold configuration is loaded.
- `--min-confidence` overrides `thresholds.cleanMinConfidence`; when neither is provided, the threshold is `0.0`. Candidates below the effective threshold are skipped.
- Preview excludes candidates whose normalized path or real parent escapes the report root. Root-contained candidates that fail safety validation remain visible in a separate safety-skipped section with status/marker evidence instead of being silently omitted.
- Apply requires exactly one normalized deletion candidate and one matching manifest entry, `safe: true`, the confidence threshold, root/real-parent containment, `sha256`, stable file and parent-directory identity, and content equality.
- On Unix, apply opens and identity-checks the canonical candidate parent plus `<root>/.kratos/clean-quarantine/`, then moves by descriptor-relative `renameat` and verifies the moved object through `openat(..., O_NOFOLLOW)`. Verified bytes remain in an invocation-unique owner-only quarantine directory; Kratos does not physically `unlink` candidate files or invocation directories. This removes the non-atomic final verify→unlink boundary against direct same-credential quarantine mutation; a failed pre-move attempt may therefore leave an empty invocation directory for manual cleanup. If post-move verification fails, Kratos does not link an unverified mutable quarantine entry back into the code tree; it reports failure, whether the original path is absent, and either the last confirmed quarantine path or that the pathname became unresolvable. Cross-filesystem `renameat` fails closed without copy/unlink fallback. Concurrent relocation/replacement of the entire report root remains outside the supported threat model.
- Apply skips schema-v2/legacy candidates without evidence, duplicate/aliased candidate paths, `safe: false`, missing/unreadable/non-regular files, direct symlinks, duplicate/missing manifest evidence, unsupported algorithms or platforms without stable identity evidence, identity/content mismatches, root escapes, and below-threshold candidates. Non-Unix destructive execution currently remains fail-closed because this descriptor-relative stable-identity boundary is unavailable.
- `clean --apply` reports the count removed from the code tree separately from every confirmed retained quarantine path, plus skipped and failed counts. The legacy public `CleanOutcome { deleted_files, skipped_files }` shape and existing function return types remain source-compatible; new `*_detailed` functions return `CleanApplyOutcome` for quarantine paths and failures. A failure with an unresolved moved quarantine pathname is reported explicitly instead of returning a false path. Per-file failures do not discard prior successful-quarantine or residue accounting. Parent directories are intentionally left in place so a mutable-parent race cannot turn convenience cleanup into an out-of-root directory removal.
- Fingerprint, file identity, and `safe` are execution-safety metadata only. They are excluded from finding identity, so the v2→v3 migration and content changes do not create diff churn.

## Evidence added in this PR

- `crates/kratos-core/tests/report_v2.rs::current_writer_keys_match_the_schema_v3_clean_safety_contract` verifies emitted writer keys against independent literal key lists and representative value/type assertions while retaining schema-v2 reader tests.
- `crates/kratos-cli/tests/cli_smoke.rs::scan_report_and_clean_work_for_demo_fixture` now includes diff summary/json smoke evidence alongside scan/report/clean evidence.
