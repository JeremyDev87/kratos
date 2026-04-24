# Kratos

[![CI](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml)

[한국어](README.md) | [English](README.en.md) | [中文](README.zh-CN.md) | [Español](README.es.md) | 日本語

デッドコードを容赦なく洗い出します。

[ライセンス](LICENSE) · [コントリビューション](CONTRIBUTING.md) · [行動規範](CODE_OF_CONDUCT.md) · [セキュリティ](SECURITY.md) · [スポンサー](https://github.com/sponsors/JeremyDev87)

Kratos は JavaScript/TypeScript プロジェクト向けの CLI ツールです。未使用ファイル、壊れた import、未使用 export、孤立したモジュールを検出し、結果を report に保存します。現在の実装は Rust core/CLI と npm launcher を組み合わせており、npm package `@jeremyfellaz/kratos` が platform ごとの optional native addon を読み込みます。

Kratos は自動削除 bot ではなく、安全なクリーンアップ手順のための分析ツールです。`clean` はデフォルトで dry-run であり、report を確認したうえで `--apply` を明示した場合だけファイルを削除します。

## 主な機能

- 未使用ファイルと孤立 component/module 候補の検出
- 壊れた内部 import の検出
- 未使用 export/import 候補の検出
- Next.js `app/` / `pages/` route entrypoint ヒューリスティック
- `tsconfig.json` / `jsconfig.json` の `baseUrl` と `paths` alias 解決
- `package.json` の `main`、`module`、`types`、`bin`、`exports` entrypoint 解決
- 保存済み report の summary、JSON、Markdown 出力
- 2つの report 間の finding 変化比較
- 信頼度しきい値を使った安全な削除候補のプレビュー

## クイックスタート

パッケージ利用時の標準 entrypoint は `npx` です。

```bash
npx @jeremyfellaz/kratos scan ./my-app
npx @jeremyfellaz/kratos report ./my-app
npx @jeremyfellaz/kratos report ./my-app --format md
npx @jeremyfellaz/kratos clean ./my-app --min-confidence 0.9
```

リストされた対象を削除すると判断した後だけ `--apply` を追加してください。

```bash
npx @jeremyfellaz/kratos clean ./my-app --apply --min-confidence 0.9
```

2つの時点の report を比較することもできます。

```bash
npx @jeremyfellaz/kratos scan ./my-app --output .kratos/before.json
# コードを整理するか branch を切り替えた後
npx @jeremyfellaz/kratos scan ./my-app --output .kratos/after.json
npx @jeremyfellaz/kratos diff ./my-app/.kratos/before.json ./my-app/.kratos/after.json
```

`scan --output` に相対パスを渡すと、スキャン対象 root から解決されます。デフォルトの保存先は `<root>/.kratos/latest-report.json` です。

## コマンド

### `kratos scan [root] [--output path] [--json]`

プロジェクトを解析し、report JSON ファイルを書き込みます。

- `root` を省略すると現在の作業ディレクトリをスキャンします。
- `--output path` は report の出力先を指定します。
- `--json` はコンソール summary の代わりに完全な report JSON を stdout に出力します。
- デフォルトの出力先は `<root>/.kratos/latest-report.json` です。

### `kratos report [report-path-or-root] [--format summary|json|md]`

保存済み report を人が読みやすい形式または元の JSON として出力します。

- `summary` はデフォルトのコンソール summary です。
- `json` は保存済み report JSON を pretty-print します。
- `md` は共有しやすい Markdown report を出力します。
- 入力がプロジェクト root の場合、Kratos は `.kratos/latest-report.json` を自動解決します。

### `kratos diff [before-report-path-or-root] [after-report-path-or-root] [--format summary|json|md]`

2つの report の finding 変化を比較します。

- デフォルト形式は `summary` です。
- `json` は introduced/resolved/persisted finding を machine-readable な形で出力します。
- `md` はレビューや issue に貼りやすい Markdown diff を出力します。
- 各入力は report ファイルパスまたはプロジェクト root にできます。

### `kratos clean [report-path-or-root] [--apply] [--min-confidence value]`

削除候補を preview するか、実際に削除します。

- デフォルト動作は dry-run です。
- `--apply` がある場合だけファイルを削除します。
- `--min-confidence value` は `0.0` から `1.0` までの信頼度しきい値です。
- `--min-confidence` を省略すると、Kratos は `kratos.config.json` の `thresholds.cleanMinConfidence` を読みます。設定がなければ `0.0` を使います。

## 出力例

`fixtures/demo-app` をスキャンすると、summary は次のような形になります。

```text
Kratos scan complete.

Root: <root>
Files scanned: 5
Entrypoints: 1
Broken imports: 1
Orphan files: 2
Dead exports: 3
Unused imports: 0
Route entrypoints: 1
Deletion candidates: 2

Saved report: <root>/.kratos/latest-report.json

Broken imports:
- <root>/src/lib/broken.ts -> ./missing-helper

Orphan files:
- <root>/src/components/DeadWidget.tsx
- <root>/src/lib/broken.ts

Dead exports:
- <root>/src/components/DeadWidget.tsx#DeadWidget
- <root>/src/lib/broken.ts#brokenFeature
- <root>/src/lib/math.ts#multiply
```

`clean --min-confidence 0.9` は、削除対象としきい値により除外された候補を分けて表示します。

```text
Kratos clean dry run.

Deletion targets: 1
- <root>/src/components/DeadWidget.tsx (confidence 0.92, Component-like module has no inbound references.)

Threshold-skipped targets: 1
- <root>/src/lib/broken.ts (confidence 0.88, Module has no inbound references and is not treated as an entrypoint.)

Re-run with --apply to delete these files.
```

同一の report を比較すると、新規または解決済みの検出結果はなく、継続している件数だけが表示されます。

```text
Kratos diff complete.

Before: <before-report>
After: <after-report>

Broken imports: introduced 0, resolved 0, persisted 1
Orphan files: introduced 0, resolved 0, persisted 2
Dead exports: introduced 0, resolved 0, persisted 3
Unused imports: introduced 0, resolved 0, persisted 0
Route entrypoints: introduced 0, resolved 0, persisted 1
Deletion candidates: introduced 0, resolved 0, persisted 2

Totals: introduced 0, resolved 0, persisted 9
```

## レポートスキーマ

現在の `scan` は `schemaVersion: 2` の report を書き込みます。

```json
{
  "schemaVersion": 2,
  "summary": {
    "filesScanned": 5,
    "entrypoints": 1,
    "brokenImports": 1,
    "orphanFiles": 2,
    "deadExports": 3,
    "unusedImports": 0,
    "routeEntrypoints": 1,
    "deletionCandidates": 2
  }
}
```

`findings` には `brokenImports`、`orphanFiles`、`deadExports`、`unusedImports`、`routeEntrypoints`、`deletionCandidates` が入ります。`graph.modules` には解析済みのモジュールパス、entrypoint 状態、import/export 件数が記録されます。

## 設定

プロジェクト root に `kratos.config.json` を置けます。JSONC 形式のコメントと trailing comma を許容します。

```json
{
  "ignore": ["storybook-static", "generated"],
  "ignorePatterns": ["src/generated/**", "!src/generated/keep.ts"],
  "entry": ["src/bootstrap.ts"],
  "roots": ["src", "app", "pages"],
  "thresholds": {
    "cleanMinConfidence": 0.85
  },
  "suppressions": [
    {
      "kind": "deadExport",
      "file": "src/components/LazyCard.tsx",
      "export": "default",
      "reason": "Loaded dynamically by route metadata."
    },
    {
      "kind": "brokenImport",
      "file": "src/legacy/shim.ts",
      "source": "./generated-shim",
      "reason": "Generated at deploy time."
    }
  ]
}
```

- `ignore`: デフォルト ignore 一覧に追加するディレクトリ名です。
- `ignorePatterns`: `.gitignore` スタイルのパスパターンです。`!` で例外を指定できます。
- Kratos はデフォルトの ignored directory の後にプロジェクト root の `.gitignore` も自動的に読み込み、その後で `ignorePatterns` を適用して例外や override を追加できます。
- `entry`: entrypoint として強制する、プロジェクト root からの相対ファイルです。
- `roots`: スキャン範囲を制限する、プロジェクト root からの相対ディレクトリです。
- `thresholds.cleanMinConfidence`: `clean` のデフォルト信頼度しきい値です。
- `suppressions`: 意図的に無視する検出結果です。`kind` は `brokenImport`、`orphanFile`、`deadExport`、`unusedImport`、`deletionCandidate` のいずれかです。

`.kratos/suppressions.json` が存在する場合、Kratos は同じ suppression 形式で読み込みます。`file` の値はプロジェクト root からの相対パスである必要があります。

## ローカル開発

必要条件:

- Node.js 18+
- npm 9+
- Rust stable toolchain

インストール:

```bash
npm install
```

推奨検証:

```bash
cargo test --workspace
npm run verify
npm run smoke
```

リポジトリ checkout では公開済み native addon package がまだ存在しない場合があるため、`npx @jeremyfellaz/kratos ...` より次のコマンドが安全です。

```bash
npm run scan -- ./fixtures/demo-app
npm run report -- ./fixtures/demo-app/.kratos/latest-report.json
npm run clean -- ./fixtures/demo-app/.kratos/latest-report.json
cargo run -p kratos-cli -- diff ./fixtures/demo-app ./fixtures/demo-app
```

## 配布構成

- root npm package は `@jeremyfellaz/kratos` です。
- CLI binary 名は `kratos` です。
- platform addon package は macOS arm64/x64、Linux x64/arm64、Windows x64 を対象にします。
- root package の `optionalDependencies` は同じリリースバージョンの platform addon package を指します。
- raw checkout には native addon がない場合がありますが、リリースパッケージの launcher は現在の platform に合う addon を読み込みます。

## リリースフロー

リリースは `vX.Y.Z` または `vX.Y.Z-prerelease.N` 形式の semver tag を基準に進みます。

- `Manual Release Bump` workflow は `package.json` と platform `optionalDependencies` を揃えるバージョンのみの PR を準備します。
- tag 作成前に、同じ commit で `cargo test --workspace`、`npm run verify`、native packaging CI が通っている必要があります。
- `Release Publish` workflow は正確な tag ref を checkout し、Node package 検証、Rust workspace test、platform native artifact build を実行します。
- platform addon npm package を先に pack/smoke/publish し、最後に root package を publish して GitHub Release を作成または更新します。
- `Release Published Follow-up` workflow は公開済みリリースに対応する publish run の成功と release asset の存在を監査します。この workflow は publish を再実行しません。

release tag push、npm publish、GitHub Release 公開のような操作は maintainer 確認後にのみ行います。

## オープンソース

Kratos は MIT ライセンスで公開されるオープンソースプロジェクトです。

- バグ報告や機能提案には GitHub Issues を利用してください。
- セキュリティ問題は公開せず、[SECURITY.md](SECURITY.md) の手順に従ってください。
- コントリビュート前に [CONTRIBUTING.md](CONTRIBUTING.md) と [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) を確認してください。
- プロジェクトを支援したい場合は、[GitHub Sponsors](https://github.com/sponsors/JeremyDev87) から支援できます。

## 注意

Kratos は保守的な静的解析とヒューリスティックを組み合わせています。dynamic import、framework convention、生成ファイル、runtime-only entrypoint はプロジェクトごとに解釈が変わることがあるため、`--apply` の前に report と diff を確認してください。
