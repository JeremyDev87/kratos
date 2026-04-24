# Kratos

[![CI](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml)

[한국어](README.md) | [English](README.en.md) | 中文 | [Español](README.es.md) | [日本語](README.ja.md)

毫不留情地清理死代码。

[许可证](LICENSE) · [贡献指南](CONTRIBUTING.md) · [行为准则](CODE_OF_CONDUCT.md) · [安全](SECURITY.md) · [赞助](https://github.com/sponsors/JeremyDev87)

Kratos 是面向 JavaScript 和 TypeScript 项目的 CLI 工具。它会找出未使用文件、断开的 import、未使用的 export 和孤立模块，并把结果写入 report。当前实现由 Rust core/CLI 和 npm launcher 组成，npm 包 `@jeremyfellaz/kratos` 会按平台加载可选的 native addon。

Kratos 是服务于安全清理流程的分析工具，不是自动删除机器人。`clean` 默认执行 dry-run，只有在你审阅 report 并显式传入 `--apply` 后才会删除文件。

## 核心能力

- 检测未使用文件以及孤立 component/module 候选项
- 检测断开的内部 import
- 检测未使用的 export 和 import 候选项
- 应用 Next.js `app/` / `pages/` route entrypoint 启发式规则
- 解析 `tsconfig.json` / `jsconfig.json` 中的 `baseUrl` 和 `paths` alias
- 解析 `package.json` 中的 `main`、`module`、`types`、`bin` 和 `exports` entrypoint
- 以 summary、JSON 或 Markdown 输出已保存的 report
- 比较两个 report 之间的 finding 变化
- 用置信度阈值预览安全删除候选项

## 快速开始

对包使用者来说，默认入口是 `npx`。

```bash
npx @jeremyfellaz/kratos scan ./my-app
npx @jeremyfellaz/kratos report ./my-app
npx @jeremyfellaz/kratos report ./my-app --format md
npx @jeremyfellaz/kratos clean ./my-app --min-confidence 0.9
```

只有在审阅 report 并决定删除列出的目标后，才添加 `--apply`。

```bash
npx @jeremyfellaz/kratos clean ./my-app --apply --min-confidence 0.9
```

你也可以比较两个时间点的 report。

```bash
npx @jeremyfellaz/kratos scan ./my-app --output .kratos/before.json
# 清理代码或切换分支后
npx @jeremyfellaz/kratos scan ./my-app --output .kratos/after.json
npx @jeremyfellaz/kratos diff ./my-app/.kratos/before.json ./my-app/.kratos/after.json
```

当 `scan --output` 收到相对路径时，该路径会从被扫描的 root 开始解析。默认保存位置是 `<root>/.kratos/latest-report.json`。

## 命令

### `kratos scan [root] [--output path] [--json]`

分析项目并写入 report JSON 文件。

- 省略 `root` 时会扫描当前工作目录。
- `--output path` 用于设置 report 输出路径。
- `--json` 会把完整 report JSON 输出到 stdout，而不是控制台 summary。
- 默认输出路径是 `<root>/.kratos/latest-report.json`。

### `kratos report [report-path-or-root] [--format summary|json|md]`

以易读格式或原始 JSON 输出已保存的 report。

- `summary` 是默认控制台 summary。
- `json` 会 pretty-print 已保存的 report JSON。
- `md` 会输出便于分享的 Markdown report。
- 如果输入是项目 root，Kratos 会自动解析 `.kratos/latest-report.json`。

### `kratos diff [before-report-path-or-root] [after-report-path-or-root] [--format summary|json|md]`

比较两个 report 之间的 finding 变化。

- 默认格式是 `summary`。
- `json` 会用 machine-readable 结构输出 introduced/resolved/persisted findings。
- `md` 会输出适合评审或 issue 的 Markdown diff。
- 每个输入都可以是 report 文件路径或项目 root。

### `kratos clean [report-path-or-root] [--apply] [--min-confidence value]`

预览删除候选项，或真正删除它们。

- 默认行为是 dry-run。
- 只有存在 `--apply` 时才会删除文件。
- `--min-confidence value` 是从 `0.0` 到 `1.0` 的置信度阈值。
- 如果省略 `--min-confidence`，Kratos 会读取 `kratos.config.json` 中的 `thresholds.cleanMinConfidence`；没有配置时使用 `0.0`。

## 输出示例

扫描 `fixtures/demo-app` 时，summary 类似如下。

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

`clean --min-confidence 0.9` 会把删除目标和因阈值被跳过的候选项分开显示。

```text
Kratos clean dry run.

Deletion targets: 1
- <root>/src/components/DeadWidget.tsx (confidence 0.92, Component-like module has no inbound references.)

Threshold-skipped targets: 1
- <root>/src/lib/broken.ts (confidence 0.88, Module has no inbound references and is not treated as an entrypoint.)

Re-run with --apply to delete these files.
```

比较两个相同 report 时，不会出现新增或已解决的检测结果，只会显示持续存在的数量。

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

## 报告 Schema

当前 `scan` 会写入 `schemaVersion: 2` report。

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

`findings` 包含 `brokenImports`、`orphanFiles`、`deadExports`、`unusedImports`、`routeEntrypoints` 和 `deletionCandidates`。`graph.modules` 会记录已分析的模块路径、entrypoint 状态以及 import/export 数量。

## 配置

你可以在项目 root 放置 `kratos.config.json`。它允许 JSONC 风格注释和 trailing comma。

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

- `ignore`：添加到默认 ignore 列表的目录名。
- `ignorePatterns`：`.gitignore` 风格的路径 pattern。可用 `!` 表示例外。
- `entry`：强制作为 entrypoint 的项目 root 相对文件。
- `roots`：限制扫描范围的项目 root 相对目录。
- `thresholds.cleanMinConfidence`：`clean` 的默认置信度阈值。
- `suppressions`：需要有意忽略的检测结果。`kind` 必须是 `brokenImport`、`orphanFile`、`deadExport`、`unusedImport` 或 `deletionCandidate` 之一。

如果存在 `.kratos/suppressions.json`，Kratos 会用相同 suppression 格式读取它。`file` 值必须是相对于项目 root 的路径。

## 本地开发

要求：

- Node.js 18+
- npm 9+
- Rust stable toolchain

安装：

```bash
npm install
```

推荐验证：

```bash
cargo test --workspace
npm run verify
npm run smoke
```

在仓库 checkout 中，已发布的 native addon 包可能还不存在，因此以下命令比 `npx @jeremyfellaz/kratos ...` 更安全。

```bash
npm run scan -- ./fixtures/demo-app
npm run report -- ./fixtures/demo-app/.kratos/latest-report.json
npm run clean -- ./fixtures/demo-app/.kratos/latest-report.json
cargo run -p kratos-cli -- diff ./fixtures/demo-app ./fixtures/demo-app
```

## 分发结构

- root npm package 是 `@jeremyfellaz/kratos`。
- CLI binary 名称是 `kratos`。
- platform addon package 覆盖 macOS arm64/x64、Linux x64/arm64 和 Windows x64。
- root package 的 `optionalDependencies` 指向相同发布版本的 platform addon package。
- raw checkout 可能没有 native addon，但发布包的 launcher 会加载当前平台对应的 addon。

## 发布流程

发布以 `vX.Y.Z` 或 `vX.Y.Z-prerelease.N` 形式的 semver tag 为准。

- `Manual Release Bump` workflow 会准备一个只改版本的 PR，使 `package.json` 和 platform `optionalDependencies` 保持一致。
- 创建 tag 前，同一个 commit 应通过 `cargo test --workspace`、`npm run verify` 和 native packaging CI。
- `Release Publish` workflow 会 checkout 精确的 tag ref，验证 Node package，运行 Rust workspace tests，并构建各平台 native artifact。
- platform addon npm package 会先被 pack、smoke-test 和 publish；最后发布 root package，并创建或更新 GitHub Release。
- `Release Published Follow-up` workflow 会审计已发布 release 是否有对应的成功 publish run 和 release assets。它不会重新执行 publish。

release tag push、npm publish、GitHub Release 发布等操作只能在维护者确认后执行。

## 开源

Kratos 是基于 MIT 许可证发布的开源项目。

- Bug 报告和功能建议请使用 GitHub Issues。
- 安全问题请不要公开提交，请按照 [SECURITY.md](SECURITY.md) 中的流程处理。
- 贡献前请先阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 和 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。
- 如果你愿意支持项目维护，可以通过 [GitHub Sponsors](https://github.com/sponsors/JeremyDev87) 赞助。

## 注意

Kratos 结合了保守静态分析和启发式规则。dynamic import、framework convention、生成文件、runtime-only entrypoint 在不同项目中可能有不同解释，因此执行 `--apply` 前请先审阅 report 和 diff。
