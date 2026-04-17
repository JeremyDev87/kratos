import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { runClean } from "../src/commands/clean.js";
import { analyzeProject } from "../src/lib/analyze.js";
import { loadProjectConfig } from "../src/lib/config.js";
import { isWithinDirectory } from "../src/lib/fs.js";
import { parseModuleSource } from "../src/lib/parser.js";
import { resolveImportTarget } from "../src/lib/resolve.js";

test("isWithinDirectory rejects prefix-only path matches", () => {
  assert.equal(
    isWithinDirectory("/tmp/app", "/tmp/application/file.ts"),
    false,
  );
  assert.equal(isWithinDirectory("/tmp/app", "/tmp/app/src/file.ts"), true);
});

test("clean --apply does not delete candidates outside the report root", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "kratos-clean-"));
  const reportRoot = path.join(tempRoot, "app");
  const outsideRoot = path.join(tempRoot, "application");
  const outsideFile = path.join(outsideRoot, "should-not-delete.txt");
  const reportPath = path.join(reportRoot, ".kratos", "latest-report.json");

  await fs.mkdir(path.dirname(reportPath), { recursive: true });
  await fs.mkdir(outsideRoot, { recursive: true });
  await fs.writeFile(outsideFile, "keep\n", "utf8");
  await fs.writeFile(
    reportPath,
    JSON.stringify({
      root: reportRoot,
      findings: {
        deletionCandidates: [{ file: outsideFile, reason: "unsafe" }],
      },
    }),
    "utf8",
  );

  const originalArgv = process.argv;
  process.argv = ["node", "kratos", "clean", reportPath];

  try {
    await runClean([reportPath, "--apply"]);
  } finally {
    process.argv = originalArgv;
  }

  assert.equal(await fileExists(outsideFile), true);
});

test("clean --apply does not follow symlinked directories outside the report root", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "kratos-clean-link-"));
  const reportRoot = path.join(tempRoot, "app");
  const outsideRoot = path.join(tempRoot, "outside");
  const outsideFile = path.join(outsideRoot, "target.txt");
  const linkPath = path.join(reportRoot, "link");
  const reportPath = path.join(reportRoot, ".kratos", "latest-report.json");

  await fs.mkdir(path.dirname(reportPath), { recursive: true });
  await fs.mkdir(outsideRoot, { recursive: true });
  await fs.writeFile(outsideFile, "keep\n", "utf8");
  await fs.symlink(outsideRoot, linkPath);
  await fs.writeFile(
    reportPath,
    JSON.stringify({
      root: reportRoot,
      findings: {
        deletionCandidates: [{ file: path.join(linkPath, "target.txt"), reason: "symlink-escape" }],
      },
    }),
    "utf8",
  );

  const originalArgv = process.argv;
  process.argv = ["node", "kratos", "clean", reportPath];

  try {
    await runClean([reportPath, "--apply"]);
  } finally {
    process.argv = originalArgv;
  }

  assert.equal(await fileExists(outsideFile), true);
});

test("clean --apply still deletes candidates when the project root itself is a symlink", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "kratos-clean-root-link-"));
  const realRoot = path.join(tempRoot, "real-app");
  const symlinkRoot = path.join(tempRoot, "linked-app");
  const deadFile = path.join(realRoot, "dead.js");
  const reportPath = path.join(symlinkRoot, ".kratos", "latest-report.json");

  await fs.mkdir(path.join(realRoot, ".kratos"), { recursive: true });
  await fs.writeFile(deadFile, "export const dead = true;\n", "utf8");
  await fs.symlink(realRoot, symlinkRoot);
  await fs.writeFile(
    reportPath,
    JSON.stringify({
      root: symlinkRoot,
      findings: {
        deletionCandidates: [{ file: path.join(symlinkRoot, "dead.js"), reason: "dead-file" }],
      },
    }),
    "utf8",
  );

  const originalArgv = process.argv;
  process.argv = ["node", "kratos", "clean", reportPath];

  try {
    await runClean([reportPath, "--apply"]);
  } finally {
    process.argv = originalArgv;
  }

  assert.equal(await fileExists(deadFile), false);
});

test("paths aliases resolve relative to baseUrl when present", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "kratos-paths-"));
  await fs.mkdir(path.join(tempRoot, "src", "foo"), { recursive: true });
  await fs.writeFile(
    path.join(tempRoot, "tsconfig.json"),
    JSON.stringify({
      compilerOptions: {
        baseUrl: "src",
        paths: {
          "@/*": ["*"],
        },
      },
    }),
    "utf8",
  );
  await fs.writeFile(path.join(tempRoot, "src", "foo", "bar.ts"), "export const bar = 1;\n");

  const config = await loadProjectConfig(tempRoot);
  const resolution = await resolveImportTarget("@/foo/bar", path.join(tempRoot, "src", "index.ts"), config);

  assert.deepEqual(resolution, {
    kind: "source",
    path: path.join(tempRoot, "src", "foo", "bar.ts"),
  });
});

test("namespace re-exports prevent source exports from being marked dead", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "kratos-namespace-"));
  await fs.mkdir(path.join(tempRoot, "src"), { recursive: true });
  await fs.writeFile(path.join(tempRoot, "kratos.config.json"), JSON.stringify({ entry: ["src/index.ts"] }), "utf8");
  await fs.writeFile(path.join(tempRoot, "src", "index.ts"), "export * as ns from './lib.ts';\n", "utf8");
  await fs.writeFile(path.join(tempRoot, "src", "lib.ts"), "export const live = 1;\n", "utf8");

  const report = await analyzeProject(tempRoot);
  assert.equal(
    report.findings.deadExports.some((entry) => entry.file.endsWith("/src/lib.ts") && entry.exportName === "live"),
    false,
  );
});

test("unused-import detection sees identifiers inside template interpolation", () => {
  const parsed = parseModuleSource(
    "import { sum } from './math';\nconst message = `value: ${sum(1, 2)}`;\n",
  );

  assert.deepEqual(parsed.unusedImports, []);
});

test("scan skips missing configured roots instead of crashing", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "kratos-roots-"));
  await fs.mkdir(path.join(tempRoot, "src"), { recursive: true });
  await fs.writeFile(
    path.join(tempRoot, "kratos.config.json"),
    JSON.stringify({ roots: ["src", "missing"], entry: ["src/index.js"] }),
    "utf8",
  );
  await fs.writeFile(path.join(tempRoot, "src", "index.js"), "export const ok = 1;\n", "utf8");

  const report = await analyzeProject(tempRoot);
  assert.equal(report.summary.filesScanned, 1);
});

test("destructured require bindings count as live imports", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "kratos-require-"));
  await fs.mkdir(path.join(tempRoot, "src"), { recursive: true });
  await fs.writeFile(
    path.join(tempRoot, "kratos.config.json"),
    JSON.stringify({ entry: ["src/index.js"] }),
    "utf8",
  );
  await fs.writeFile(
    path.join(tempRoot, "src", "index.js"),
    "const { helper } = require('./helper');\nmodule.exports = { run: () => helper() };\n",
    "utf8",
  );
  await fs.writeFile(path.join(tempRoot, "src", "helper.js"), "exports.helper = () => 1;\n", "utf8");

  const report = await analyzeProject(tempRoot);
  assert.equal(report.findings.orphanFiles.some((entry) => entry.file.endsWith("/src/helper.js")), false);
  assert.equal(report.findings.deadExports.some((entry) => entry.file.endsWith("/src/helper.js")), false);
});

test("nested require destructuring is treated conservatively to avoid false dead-code findings", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "kratos-require-nested-"));
  await fs.mkdir(path.join(tempRoot, "src"), { recursive: true });
  await fs.writeFile(
    path.join(tempRoot, "kratos.config.json"),
    JSON.stringify({ entry: ["src/index.js"] }),
    "utf8",
  );
  await fs.writeFile(
    path.join(tempRoot, "src", "index.js"),
    "const { helper: { nested } } = require('./helper');\nmodule.exports = { run: () => nested() };\n",
    "utf8",
  );
  await fs.writeFile(
    path.join(tempRoot, "src", "helper.js"),
    "exports.helper = { nested: () => 1 };\n",
    "utf8",
  );

  const report = await analyzeProject(tempRoot);
  assert.equal(report.findings.orphanFiles.some((entry) => entry.file.endsWith("/src/helper.js")), false);
  assert.equal(report.findings.deadExports.some((entry) => entry.file.endsWith("/src/helper.js")), false);
});

test("require destructuring with ternary defaults does not misparse alias separators", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "kratos-require-ternary-"));
  await fs.mkdir(path.join(tempRoot, "src"), { recursive: true });
  await fs.writeFile(
    path.join(tempRoot, "kratos.config.json"),
    JSON.stringify({ entry: ["src/index.js"] }),
    "utf8",
  );
  await fs.writeFile(
    path.join(tempRoot, "src", "index.js"),
    "const { a = cond ? fallback() : other() } = require('./helper');\nmodule.exports = { run: () => a };\n",
    "utf8",
  );
  await fs.writeFile(path.join(tempRoot, "src", "helper.js"), "exports.a = 1;\n", "utf8");

  const report = await analyzeProject(tempRoot);
  assert.equal(report.findings.deadExports.some((entry) => entry.file.endsWith("/src/helper.js")), false);
});

async function fileExists(filePath) {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}
