# Kratos

[![CI](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml)

[한국어](README.md) | [English](README.en.md) | [中文](README.zh-CN.md) | [Español](README.es.md) | 日本語

Destroy dead code ruthlessly.

[License](LICENSE) · [Contributing](CONTRIBUTING.md) · [Code of Conduct](CODE_OF_CONDUCT.md) · [Security](SECURITY.md) · [Sponsor](https://github.com/sponsors/JeremyDev87)

Kratos は、プロジェクト内に潜んでいるデッドコード、未使用ファイル、壊れた import、孤立したモジュールを見つけ、安全に削除できる候補を提案する CLI ツールです。現在の配布構成は Rust core/CLI と npm launcher を組み合わせたもので、保守的な解析と安全なクリーンアップ提案に重点を置いています。

## 主な機能

- 未使用ファイルの検出
- dead export の検出
- broken import の検出
- 孤立モジュール / 孤立コンポーネントの検出
- 安全な削除候補の提案
- コードベースのスリム化レポート生成

## クイックスタート

パッケージ利用時の標準エントリーポイントは `npx` です。

```bash
npx @jeremyfellaz/kratos scan ./your-project
npx @jeremyfellaz/kratos report ./your-project/.kratos/latest-report.json
npx @jeremyfellaz/kratos clean ./your-project/.kratos/latest-report.json
```

- `scan` はデフォルトで `.kratos/latest-report.json` を生成します
- `clean` はデフォルトで dry-run であり、実際に削除するのは `--apply` を付けたときだけです

## ローカル開発

リポジトリのチェックアウトでは Rust CLI と npm script を使って開発します。

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

リポジトリ内で CLI を確認するときは次を使います。

```bash
npm run scan -- ./fixtures/demo-app
npm run report -- ./fixtures/demo-app/.kratos/latest-report.json
npm run clean -- ./fixtures/demo-app/.kratos/latest-report.json
```

チェックアウト状態では公開済み native addon package がまだ存在しないことがあるため、`npx @jeremyfellaz/kratos ...` より上記のコマンドか `cargo run -p kratos-cli -- ...` を使うのが安全です。

## コマンド

### `kratos scan [root]`

プロジェクトをスキャンし、最新レポートを保存します。

- デフォルト出力先: `<root>/.kratos/latest-report.json`
- `--output <path>`: レポートの出力先を指定
- `--json`: コンソール要約の代わりに完全な JSON を stdout に出力

### `kratos report [report-path-or-root]`

保存済みレポートを summary、JSON、Markdown 形式で出力します。

- `--format summary`: デフォルトの要約出力
- `--format json`: 生の JSON 出力
- `--format md`: Markdown レポート出力
- レポートパスの代わりにプロジェクト root を渡すと最新レポートを自動解決します

### `kratos clean [report-path-or-root]`

削除候補を表示するか、実際に削除します。

- デフォルト動作は dry-run です
- `--apply`: 実際の削除を実行
- レポートパスの代わりにプロジェクト root を渡すと最新レポートを自動解決します

## レポートスキーマ

現在の Rust scan 出力は `schemaVersion: 2` を書き込みます。

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

- デフォルトの保存先は `.kratos/latest-report.json` です
- `report` と `clean` は保存済みレポートパスまたはプロジェクト root のどちらでも受け付けます
- Markdown 出力には broken imports、orphan files、dead exports、route entrypoints、deletion candidates が含まれます

## 現在の検出範囲

- Rust analyzer と Oxc ベースの JS/TS import/export 解析
- relative import / require / dynamic import
- `tsconfig.json` / `jsconfig.json` の `baseUrl` と `paths`
- Next.js `app/` / `pages/` route entrypoint ヒューリスティック
- `package.json` の `main`、`module`、`bin`、`exports` エントリーポイント
- orphan file / orphan component 候補
- dead export 候補
- unused import 候補
- broken internal imports

## 設定

必要に応じて `kratos.config.json` を追加できます。

```json
{
  "ignore": ["storybook-static", "generated"],
  "entry": ["src/bootstrap.ts"],
  "roots": ["src", "app", "pages"]
}
```

- `ignore`: 追加で無視するディレクトリ名
- `entry`: エントリーポイントとして強制指定するファイルパス
- `roots`: スキャン対象を制限するフォルダ

## こんなチームにおすすめ

- 古い React / Next.js プロジェクト
- 機能リリースが多く、コードが蓄積しているチーム
- リファクタリングのタイミングを探しているチーム

## リリースフロー

Kratos の release automation は `v0.2.0-alpha.1` や `v0.2.0` のようなセマンティックバージョンタグで動きます。

Alpha 候補の準備:

- root package version は `0.2.0-alpha.1` のままにします
- タグ作成前に `cargo test --workspace`、`npm run verify`、`npm run smoke`、fixture ベースの `scan/report/clean` smoke を完了します
- alpha タグの作成と push は maintainer の確認後に行います

[release workflow](.github/workflows/release.yml) はタグ push、または既存タグを指定した手動実行で起動し、次を行います。

- release metadata を解決し、prerelease なら npm dist-tag を `next` にします
- Node package verification と Rust workspace test を分けて実行します
- macOS arm64/x64、Linux x64/arm64、Windows x64 向け native artifact をビルドします
- platform addon npm package を pack と smoke してから先に publish します
- 最後に root `kratos` package を publish し、GitHub Release を作成します

Stable 昇格:

- alpha 検証後、`package.json` を `0.2.0-alpha.1` から `0.2.0` に上げる version-only の release-prep commit を別で作ります
- stable タグ `v0.2.0` はその次の段階で別途作成し、npm `latest` に配布されます

推奨の publish 構成は npm Trusted Publishing（OIDC）です。必要であればリポジトリの `NPM_TOKEN` secret fallback も利用できます。

## オープンソース

Kratos は MIT ライセンスで公開されるオープンソースプロジェクトです。

- バグ報告や機能提案には GitHub Issues を利用してください。
- セキュリティ問題は公開せず、[SECURITY.md](SECURITY.md) の手順に従ってください。
- コントリビュート前に [CONTRIBUTING.md](CONTRIBUTING.md) と [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) を確認してください。
- プロジェクトを支援したい場合は、[GitHub Sponsors](https://github.com/sponsors/JeremyDev87) から支援できます。

## 注意

現在の alpha は Rust core と Oxc ベースの解析を使っていますが、entrypoint 判定と安全な削除候補の選定には依然として保守的なヒューリスティックが含まれます。`--apply` の前に必ずレポートを確認してください。
