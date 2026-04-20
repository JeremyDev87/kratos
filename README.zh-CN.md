# Kratos

[![CI](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml)

[한국어](README.md) | [English](README.en.md) | 中文 | [Español](README.es.md) | [日本語](README.ja.md)

Destroy dead code ruthlessly.

[License](LICENSE) · [Contributing](CONTRIBUTING.md) · [Code of Conduct](CODE_OF_CONDUCT.md) · [Security](SECURITY.md) · [Sponsor](https://github.com/sponsors/JeremyDev87)

Kratos 是一个 CLI 工具，用来找出项目中隐藏的死代码，包括未使用的文件、断开的 import 和孤立模块，并给出相对安全的删除候选项。当前的分发结构由 Rust core/CLI 与 npm launcher 组成，重点放在保守分析和安全清理建议上。

## 核心能力

- 检测未使用文件
- 检测 dead export
- 检测 broken import
- 检测孤立模块和孤立组件
- 提供安全删除候选项
- 生成代码库瘦身报告

## 快速开始

对包使用者来说，默认入口是 `npx`。

```bash
npx kratos scan ./your-project
npx kratos report ./your-project/.kratos/latest-report.json
npx kratos clean ./your-project/.kratos/latest-report.json
```

- `scan` 默认会生成 `.kratos/latest-report.json`
- `clean` 默认是 dry-run，只有加上 `--apply` 才会真正删除文件

## 本地开发

在仓库 checkout 中，请使用 Rust CLI 和 npm script。

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

在仓库里直接验证 CLI 时，请使用：

```bash
npm run scan -- ./fixtures/demo-app
npm run report -- ./fixtures/demo-app/.kratos/latest-report.json
npm run clean -- ./fixtures/demo-app/.kratos/latest-report.json
```

在 checkout 状态下，已发布的原生 addon 包可能还不存在，因此更适合使用上面的命令或 `cargo run -p kratos-cli -- ...`，而不是 `npx kratos ...`。

## 命令

### `kratos scan [root]`

扫描项目，并写入最新报告。

- 默认输出路径：`<root>/.kratos/latest-report.json`
- `--output <path>`：自定义报告输出路径
- `--json`：把完整 JSON 输出到 stdout，而不是控制台摘要

### `kratos report [report-path-or-root]`

以 summary、JSON 或 Markdown 形式输出已保存的报告。

- `--format summary`：默认摘要输出
- `--format json`：原始 JSON 输出
- `--format md`：Markdown 报告输出
- 如果传入的是项目根目录而不是报告路径，Kratos 会自动解析最新报告

### `kratos clean [report-path-or-root]`

显示删除候选项，或执行实际删除。

- 默认是 dry-run
- `--apply`：执行实际删除
- 如果传入的是项目根目录而不是报告路径，Kratos 会自动解析最新报告

## 报告 Schema

当前 Rust scan 输出会写入 `schemaVersion: 2`。

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

- 默认保存位置是 `.kratos/latest-report.json`
- `report` 和 `clean` 都可以接收已保存的报告路径或项目根目录
- Markdown 输出会汇总 broken imports、orphan files、dead exports、route entrypoints 和 deletion candidates

## 当前覆盖范围

- 使用 Rust analyzer 和基于 Oxc 的 JS/TS import/export 解析
- relative import / require / dynamic import
- `tsconfig.json` / `jsconfig.json` 中的 `baseUrl` 与 `paths`
- Next.js `app/` / `pages/` route entrypoint 启发式识别
- `package.json` 中的 `main`、`module`、`bin`、`exports` 入口点
- orphan file / orphan component 候选
- dead export 候选
- unused import 候选
- broken internal imports

## 配置

你也可以选择添加 `kratos.config.json`。

```json
{
  "ignore": ["storybook-static", "generated"],
  "entry": ["src/bootstrap.ts"],
  "roots": ["src", "app", "pages"]
}
```

- `ignore`：额外忽略的目录名
- `entry`：强制作为入口点的文件路径
- `roots`：限制扫描范围的目录

## 适合哪些团队

- 老旧的 React / Next.js 项目
- 功能上线很多、代码不断堆积的团队
- 正在寻找重构时机的团队

## 发布流程

Kratos 的发布自动化由 `v0.2.0-alpha.1`、`v0.2.0` 这类语义化版本标签驱动。

Alpha 候选准备：

- root package version 保持在 `0.2.0-alpha.1`
- 打标签前先完成 `cargo test --workspace`、`npm run verify`、`npm run smoke`，以及基于 fixture 的 `scan/report/clean` smoke
- alpha 标签的创建和 push 需在维护者确认后再执行

[release workflow](.github/workflows/release.yml) 会在标签 push 或指定已有标签的手动触发时运行，然后：

- 解析 release metadata，并把 prerelease 映射到 npm dist-tag `next`
- 分别验证 Node package 和 Rust workspace
- 构建 macOS arm64/x64、Linux x64/arm64、Windows x64 的原生产物
- 先打包并 smoke-test 各平台 addon npm package，再发布它们
- 最后发布 root `kratos` package，并创建 GitHub Release

Stable 升级：

- alpha 验证通过后，在一个仅修改版本号的 release-prep commit 中，把 `package.json` 从 `0.2.0-alpha.1` 提升到 `0.2.0`
- stable 标签 `v0.2.0` 在后续单独步骤中创建，并发布到 npm `latest`

推荐的发布方式是 npm Trusted Publishing（OIDC）。需要时也可以回退到仓库里的 `NPM_TOKEN` secret。

## 开源

Kratos 是基于 MIT 许可证发布的开源项目。

- Bug 报告和功能建议请使用 GitHub Issues。
- 安全问题请不要公开提交，按 [SECURITY.md](SECURITY.md) 中的流程处理。
- 贡献前请先阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 和 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。
- 如果你愿意支持项目维护，可以通过 [GitHub Sponsors](https://github.com/sponsors/JeremyDev87) 赞助。

## 注意

当前 alpha 使用 Rust core 和基于 Oxc 的解析，但 entrypoint 判定与安全删除候选仍然包含保守启发式。执行 `--apply` 之前，请务必先审阅报告。
