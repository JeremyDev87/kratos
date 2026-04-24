# Kratos Planning Index

이 디렉터리는 현재 `master` 기준으로 아직 남은 제품 계획만 유지한다. 완료된 Rust 전환 계획과 JS-era 계획은 active docs에서 제거했다.

## Active Documents

- `01-wow-roadmap-overview.md`
  - 현재 구현 baseline과 남은 roadmap phase index
- `03-report-history-and-presentation.md`
  - 남은 HTML report 작업
- `04-sweep-experience.md`
  - 남은 `kratos sweep` guided cleanup 작업
- `05-workspace-cache-and-watch.md`
  - workspace scan, incremental cache, polling watch 작업

## Current Baseline Snapshot

- package: `@jeremyfellaz/kratos` `0.3.2`
- commands: `scan`, `report`, `diff`, `clean`
- report formats: `summary`, `json`, `md`
- implemented controls: `.gitignore`, `ignorePatterns`, suppression, clean confidence threshold
- release actions are out of scope for these plans

## Removed From Active Planning

- `docs/plan/*` Rust migration docs: completed and stale against the current Rust checkout.
- `docs/plans/00-cli-help-modularization.md`: JS-era `src/cli.js` plan, no longer applicable.
- `docs/plans/02-analysis-trust-and-controls.md`: dynamic usage, suppression, and clean threshold are already implemented.
