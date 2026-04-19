# Kratos

[![CI](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml)

[한국어](README.md) | [English](README.en.md) | [中文](README.zh-CN.md) | [Español](README.es.md) | 日本語

Destroy dead code ruthlessly.

[License](LICENSE) · [Contributing](CONTRIBUTING.md) · [Code of Conduct](CODE_OF_CONDUCT.md) · [Security](SECURITY.md) · [Sponsor](https://github.com/sponsors/JeremyDev87)

Kratos は、プロジェクト内に潜んでいるデッドコード、未使用ファイル、壊れた import、孤立したモジュールを見つけ、安全に削除できる候補を提案する CLI ツールです。レガシーが積み上がるほどコードベースは重くなり、保守コストも増えていきます。Kratos は不要な痕跡を可視化し、コードベースを再び引き締めることに集中します。

## 主な機能

- 未使用ファイルの検出
- dead export の検出
- broken import の検出
- 孤立モジュール / 孤立コンポーネントの検出
- 安全な削除候補の提案
- コードベースのスリム化レポート生成

## クイックスタート

```bash
npm install
npx kratos scan
npx kratos report
npx kratos clean
```

ローカル開発中は、以下のように実行することもできます。

```bash
npm run scan -- ./fixtures/demo-app
npm run report -- ./fixtures/demo-app/.kratos/latest-report.json
npm run clean -- ./fixtures/demo-app/.kratos/latest-report.json
```

## コマンド

### `kratos scan [root]`

プロジェクトをスキャンし、分析結果を `.kratos/latest-report.json` に保存します。

オプション:

- `--output <path>`: レポートの出力先を指定
- `--json`: コンソール要約の代わりに完全な JSON を出力

### `kratos report [report-path-or-root]`

保存済みの最新レポートを読み込み、人が読みやすい形式で出力します。

オプション:

- `--format summary`: デフォルトの要約出力
- `--format json`: 生の JSON 出力
- `--format md`: Markdown レポート出力

### `kratos clean [report-path-or-root]`

削除候補を表示するか、実際に削除します。

オプション:

- `--apply`: 実際の削除を実行

デフォルト動作は dry-run です。`--apply` を付けない限り、ファイルは削除されません。

## 現在の MVP が検出できるもの

- JS / JSX / TS / TSX / MJS / CJS のファイルグラフ
- relative import / require / dynamic import
- `tsconfig.json` / `jsconfig.json` の `baseUrl` と `paths`
- Next.js `app/` / `pages/` のエントリーファイルのヒューリスティック
- package.json の `main`、`module`、`bin`、`exports` エントリーポイント
- orphan file / orphan component 候補
- dead export 候補
- unused import 候補
- broken internal import

## レポート例

```bash
$ npm run scan -- ./fixtures/demo-app
Kratos scan complete.

Root: /.../fixtures/demo-app
Files scanned: 5
Entrypoints: 1
Broken imports: 1
Orphan files: 2
Dead exports: 3
Unused imports: 0
Deletion candidates: 2

Saved report: /.../fixtures/demo-app/.kratos/latest-report.json
```

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

## リリース

Kratos は `v0.2.0-alpha.1` や `v1.0.0` のようなセマンティックバージョンタグでリリースします。

```bash
npm version 0.2.0-alpha.1 --no-git-tag-version
git add package*.json
git commit -m "chore: release v0.2.0-alpha.1"
git tag v0.2.0-alpha.1
git push origin HEAD
git push origin v0.2.0-alpha.1
```

タグが push されると、[release workflow](.github/workflows/release.yml) が以下を実行します。

- `npm run verify` を実行
- npm 配布用 tarball を生成
- stable リリースは npm `latest`、prerelease は npm `next` に publish
- GitHub Release を作成し tarball を添付

推奨構成は npm Trusted Publishing（OIDC）です。まだ設定していない場合は、リポジトリの `NPM_TOKEN` secret による fallback も可能です。

## オープンソース

Kratos は MIT ライセンスで公開されるオープンソースプロジェクトです。

- バグ報告や機能提案には GitHub Issues を利用してください。
- セキュリティ問題は公開せず、[SECURITY.md](SECURITY.md) の手順に従ってください。
- コントリビュート前に [CONTRIBUTING.md](CONTRIBUTING.md) と [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) を確認してください。
- プロジェクトを支援したい場合は、[GitHub Sponsors](https://github.com/sponsors/JeremyDev87) から支援できます。

## 注意

このバージョンは AST ベースの解析器ではなく、ヒューリスティックな MVP です。大規模なプロジェクトを素早く確認することに最適化されているため、削除前には必ずレポートを確認してください。
