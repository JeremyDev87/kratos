import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import assert from "node:assert/strict";

import {
  isDirectExecution,
  resolveAddonPackageName,
  runLauncher,
} from "../bin/kratos.js";

const skipDirectExecutableTests =
  process.platform === "win32"
    ? "Windows executes npm bins through generated cmd shims, not JS shebang files."
    : false;

test("resolveAddonPackageName maps supported targets", () => {
  assert.equal(resolveAddonPackageName("darwin", "arm64"), "@jeremyfellaz/kratos-darwin-arm64");
  assert.equal(resolveAddonPackageName("darwin", "x64"), "@jeremyfellaz/kratos-darwin-x64");
  assert.equal(resolveAddonPackageName("linux", "x64"), "@jeremyfellaz/kratos-linux-x64-gnu");
  assert.equal(resolveAddonPackageName("linux", "arm64"), "@jeremyfellaz/kratos-linux-arm64-gnu");
  assert.equal(resolveAddonPackageName("win32", "x64"), "@jeremyfellaz/kratos-win32-x64-msvc");
  assert.throws(
    () => resolveAddonPackageName("linux", "ppc64"),
    /Unsupported platform\/arch/,
  );
});

test("runLauncher forwards argv to runCli and returns its exit code", () => {
  let receivedArgs = null;
  const stderr = captureStream();

  const exitCode = runLauncher(["node", "bin/kratos.js", "scan", "fixtures/demo-app"], {
    platform: "darwin",
    arch: "arm64",
    requireFn(packageName) {
      assert.equal(packageName, "@jeremyfellaz/kratos-darwin-arm64");
      return {
        runCli(args) {
          receivedArgs = args;
          return 7;
        },
      };
    },
    stderr,
  });

  assert.equal(exitCode, 7);
  assert.deepEqual(receivedArgs, ["scan", "fixtures/demo-app"]);
  assert.equal(stderr.read(), "");
});

test("runLauncher formats missing addon failures with Korean Kratos prefix", () => {
  const stderr = captureStream();

  const exitCode = runLauncher(["node", "bin/kratos.js", "scan"], {
    platform: "linux",
    arch: "x64",
    requireFn() {
      throw new Error("Cannot find module '@jeremyfellaz/kratos-linux-x64-gnu'");
    },
    stderr,
  });

  assert.equal(exitCode, 1);
  assert.match(
    stderr.read(),
    /Kratos 실행 실패: Failed to load native addon package @jeremyfellaz\/kratos-linux-x64-gnu:/,
  );
});

test("runLauncher fails when binding does not export runCli", () => {
  const stderr = captureStream();

  const exitCode = runLauncher(["node", "bin/kratos.js", "scan"], {
    platform: "darwin",
    arch: "x64",
    requireFn() {
      return {};
    },
    stderr,
  });

  assert.equal(exitCode, 1);
  assert.match(stderr.read(), /does not export runCli/);
});

test("isDirectExecution only matches the active entry file", () => {
  assert.equal(
    isDirectExecution("file:///repo/bin/kratos.js", ["/usr/bin/node", "/repo/bin/kratos.js"]),
    true,
  );
  assert.equal(
    isDirectExecution("file:///repo/bin/kratos.js", ["/usr/bin/node", "/repo/other.js"]),
    false,
  );
});

test(
  "symlinked launcher execution still enters runLauncher",
  { skip: skipDirectExecutableTests },
  () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "kratos-launcher-link-"));
    const launcherPath = prepareIsolatedLauncher(tempRoot);
    const linkPath = path.join(tempRoot, "kratos.js");
    fs.symlinkSync(launcherPath, linkPath, "file");

    const result = spawnSync(linkPath, ["scan"], {
      encoding: "utf8",
    });

    assert.equal(result.status, 1);
    assert.match(result.stderr, expectedMissingAddonPattern());
  },
);

test(
  "launcher file executes directly through its shebang",
  { skip: skipDirectExecutableTests },
  () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "kratos-launcher-direct-"));
    const launcherPath = prepareIsolatedLauncher(tempRoot);
    const result = spawnSync(launcherPath, ["scan"], {
      encoding: "utf8",
    });

    assert.equal(result.status, 1);
    assert.match(result.stderr, expectedMissingAddonPattern());
  },
);

function prepareIsolatedLauncher(tempRoot) {
  const launcherDir = path.join(tempRoot, "bin");
  const launcherPath = path.join(launcherDir, "kratos.js");

  fs.mkdirSync(launcherDir, { recursive: true });
  fs.writeFileSync(
    path.join(tempRoot, "package.json"),
    JSON.stringify(
      {
        type: "module",
      },
      null,
      2,
    ) + "\n",
  );
  fs.copyFileSync(path.join(process.cwd(), "bin/kratos.js"), launcherPath);
  fs.chmodSync(launcherPath, 0o755);

  return launcherPath;
}

function captureStream() {
  let buffer = "";

  return {
    write(chunk) {
      buffer += String(chunk);
      return true;
    },
    read() {
      return buffer;
    },
  };
}

function escapeForRegex(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function expectedMissingAddonPattern() {
  const expectedPackageName = resolveAddonPackageName(process.platform, process.arch);

  return new RegExp(
    `Kratos 실행 실패: Failed to load native addon package ${escapeForRegex(expectedPackageName)}:`,
  );
}
