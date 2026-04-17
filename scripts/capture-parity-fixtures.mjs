import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const cliPath = path.join(repoRoot, "src", "cli.js");
const parityRoot = path.join(repoRoot, "fixtures", "parity");
const fixtures = [
  {
    id: "demo-app",
    sourceRoot: path.join(repoRoot, "fixtures", "demo-app"),
    outputRoot: path.join(parityRoot, "demo-app"),
  },
];

await mkdir(parityRoot, { recursive: true });
await writeManifest();

for (const fixture of fixtures) {
  await captureFixture(fixture);
}

async function writeManifest() {
  const manifest = {
    version: 1,
    generatedBy: "node ./scripts/capture-parity-fixtures.mjs",
    fixtures: fixtures.map((fixture) => ({
      id: fixture.id,
      sourceRoot: toRepoRelative(fixture.sourceRoot),
      outputs: {
        report: `fixtures/parity/${fixture.id}/latest-report.v1.json`,
        summary: `fixtures/parity/${fixture.id}/report-summary.txt`,
        markdown: `fixtures/parity/${fixture.id}/report-markdown.md`,
      },
    })),
  };

  await writeFile(
    path.join(parityRoot, "manifest.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
}

async function captureFixture(fixture) {
  await mkdir(fixture.outputRoot, { recursive: true });

  const tempRoot = await mkdtemp(path.join(os.tmpdir(), "kratos-parity-"));
  const tempReportPath = path.join(tempRoot, `${fixture.id}-latest-report.json`);

  try {
    runCli(["scan", fixture.sourceRoot, "--output", tempReportPath]);

    const summaryOutput = runCli([
      "report",
      tempReportPath,
      "--format",
      "summary",
    ]);
    const markdownOutput = runCli([
      "report",
      tempReportPath,
      "--format",
      "md",
    ]);

    const rawReport = JSON.parse(await readFile(tempReportPath, "utf8"));
    const normalizedReport = normalizeValue(rawReport, fixture.sourceRoot, tempReportPath);

    await writeFile(
      path.join(fixture.outputRoot, "latest-report.v1.json"),
      `${JSON.stringify(normalizedReport, null, 2)}\n`,
    );
    await writeFile(
      path.join(fixture.outputRoot, "report-summary.txt"),
      `${normalizeText(summaryOutput, fixture.sourceRoot, tempReportPath)}\n`,
    );
    await writeFile(
      path.join(fixture.outputRoot, "report-markdown.md"),
      `${normalizeText(markdownOutput, fixture.sourceRoot, tempReportPath)}\n`,
    );
  } finally {
    await rm(tempRoot, { recursive: true, force: true });
  }
}

function runCli(args) {
  const result = spawnSync(process.execPath, [cliPath, ...args], {
    cwd: repoRoot,
    encoding: "utf8",
  });

  if (result.status !== 0) {
    const stderr = result.stderr?.trim() ? `\n${result.stderr.trim()}` : "";
    throw new Error(`Failed to run kratos ${args.join(" ")}${stderr}`);
  }

  return result.stdout.trimEnd();
}

function normalizeValue(value, fixtureRoot, reportPath) {
  if (Array.isArray(value)) {
    return value.map((entry) => normalizeValue(entry, fixtureRoot, reportPath));
  }

  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([key, entry]) => [
        key,
        normalizeValue(normalizeScalar(key, entry), fixtureRoot, reportPath),
      ]),
    );
  }

  if (typeof value === "string") {
    return normalizeText(value, fixtureRoot, reportPath);
  }

  return value;
}

function normalizeText(text, fixtureRoot, reportPath) {
  return toPosixPath(text)
    .replaceAll(toPosixPath(reportPath), "<REPORT>")
    .replaceAll(toPosixPath(fixtureRoot), "<ROOT>")
    .replaceAll(/Generated: .+/g, "Generated: <GENERATED_AT>");
}

function normalizeScalar(key, value) {
  if (key === "generatedAt") {
    return "<GENERATED_AT>";
  }

  return value;
}

function toRepoRelative(absolutePath) {
  return toPosixPath(path.relative(repoRoot, absolutePath));
}

function toPosixPath(value) {
  return String(value).replaceAll("\\", "/");
}
