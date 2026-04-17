import test from "node:test";
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const cliPath = fileURLToPath(new URL("../src/cli.js", import.meta.url));

test("root help is generated from command metadata", async () => {
  const { stdout, stderr } = await execFileAsync(process.execPath, [cliPath, "--help"]);

  assert.equal(stderr, "");
  assert.match(stdout, /kratos scan \[root] \[--output path] \[--json]/);
  assert.match(stdout, /kratos report \[report-path-or-root] \[--format summary\|json\|md]/);
  assert.match(stdout, /kratos clean \[report-path-or-root] \[--apply]/);
  assert.match(stdout, /scan\s+Analyze a codebase and save the latest report\./);
  assert.match(stdout, /clean\s+Show deletion candidates or delete them with --apply\./);
});

test("command help is served from its command module", async () => {
  const { stdout, stderr } = await execFileAsync(process.execPath, [cliPath, "scan", "--help"]);

  assert.equal(stderr, "");
  assert.match(stdout, /scan command/);
  assert.match(stdout, /Analyze a codebase and save the latest report\./);
  assert.match(stdout, /kratos scan \[root] \[--output path] \[--json]/);
  assert.match(stdout, /Run `kratos --help` to see every command\./);
});

test("command short help works when -h is the only subcommand argument", async () => {
  const { stdout, stderr } = await execFileAsync(process.execPath, [cliPath, "scan", "-h"]);

  assert.equal(stderr, "");
  assert.match(stdout, /scan command/);
  assert.match(stdout, /kratos scan \[root] \[--output path] \[--json]/);
});

test("command help does not steal -h when it is a command argument value", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "kratos-cli-help-"));
  const outputPath = path.join(tempRoot, "-h");
  const { stdout, stderr } = await execFileAsync(process.execPath, [
    cliPath,
    "scan",
    tempRoot,
    "--output",
    "-h",
  ]);

  assert.equal(stderr, "");
  assert.match(stdout, /Kratos scan complete\./);
  assert.doesNotMatch(stdout, /scan command/);
  await fs.access(outputPath);
});

test("unknown commands print root help to stderr and exit non-zero", async () => {
  await assert.rejects(
    execFileAsync(process.execPath, [cliPath, "explode"]),
    (error) => {
      assert.equal(error.code, 1);
      assert.match(error.stderr, /Unknown command: explode/);
      assert.match(error.stderr, /Usage:/);
      assert.match(error.stderr, /kratos clean \[report-path-or-root] \[--apply]/);
      return true;
    },
  );
});
