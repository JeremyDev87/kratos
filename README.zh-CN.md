# Kratos

[한국어](README.md) | [English](README.en.md) | 中文 | [Español](README.es.md) | [日本語](README.ja.md)

Destroy dead code ruthlessly.

[License](LICENSE) · [Contributing](CONTRIBUTING.md) · [Code of Conduct](CODE_OF_CONDUCT.md) · [Security](SECURITY.md) · [Sponsor](https://github.com/sponsors/JeremyDev87)

Kratos 是一个 CLI 工具，用来找出项目中隐藏的死代码，包括未使用的文件、断开的 import 和孤立模块，并给出相对安全的删除候选项。随着遗留代码不断堆积，代码库会变得越来越沉重，维护成本也会越来越高。Kratos 的目标就是把这些不再需要的痕迹暴露出来，让代码库重新变得轻快。

## 核心能力

- 检测未使用文件
- 检测 dead export
- 检测 broken import
- 检测孤立模块和孤立组件
- 提供安全删除候选项
- 生成代码库瘦身报告

## 快速开始

```bash
npm install
npx kratos scan
npx kratos report
npx kratos clean
```

在本地开发时，也可以这样运行：

```bash
node ./src/cli.js scan
node ./src/cli.js report
node ./src/cli.js clean
```

## 命令

### `kratos scan [root]`

扫描项目，并将分析结果写入 `.kratos/latest-report.json`。

选项：

- `--output <path>`：自定义报告输出路径
- `--json`：输出完整 JSON，而不是控制台摘要

### `kratos report [report-path-or-root]`

读取最近保存的报告，并以更适合阅读的格式输出。

选项：

- `--format summary`：默认摘要输出
- `--format json`：原始 JSON 输出
- `--format md`：Markdown 报告输出

### `kratos clean [report-path-or-root]`

显示删除候选项，或执行实际删除。

选项：

- `--apply`：执行实际删除

默认行为是 dry-run。没有 `--apply` 时，不会删除任何文件。

## 当前 MVP 可以检测的内容

- JS / JSX / TS / TSX / MJS / CJS 文件图
- relative import / require / dynamic import
- `tsconfig.json` / `jsconfig.json` 中的 `baseUrl` 与 `paths`
- Next.js `app/` / `pages/` 入口文件启发式识别
- package.json 中的 `main`、`module`、`bin`、`exports` 入口点
- orphan file / orphan component 候选
- dead export 候选
- unused import 候选
- broken internal import

## 报告示例

```bash
$ node ./src/cli.js scan ./fixtures/demo-app
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

## 开源

Kratos 是基于 MIT 许可证发布的开源项目。

- Bug 报告和功能建议请使用 GitHub Issues。
- 安全问题请不要公开提交，按 [SECURITY.md](SECURITY.md) 中的流程处理。
- 贡献前请先阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 和 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。
- 如果你愿意支持项目维护，可以通过 [GitHub Sponsors](https://github.com/sponsors/JeremyDev87) 赞助。

## 注意

当前版本还是基于启发式规则的 MVP，并不是 AST 级别的分析器。它的重点是快速扫描大型项目，因此在执行删除之前，仍然建议你先审阅报告。
