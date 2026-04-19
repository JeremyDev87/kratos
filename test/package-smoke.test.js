import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import fsp from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath, pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";
import crypto from "node:crypto";

import { resolveAddonPackageName } from "../bin/kratos.js";

const require = createRequire(import.meta.url);
const packageJson = require("../package.json");
const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
let cmdShimPromise;
const ADDON_TARGETS = [
  { platform: "darwin", arch: "arm64", os: "darwin", cpu: "arm64" },
  { platform: "darwin", arch: "x64", os: "darwin", cpu: "x64" },
  { platform: "linux", arch: "x64", os: "linux", cpu: "x64" },
  { platform: "linux", arch: "arm64", os: "linux", cpu: "arm64" },
  { platform: "win32", arch: "x64", os: "win32", cpu: "x64" },
];

test("root npm pack dry-run excludes src runtime files", () => {
  const result = spawnSync(
    "npm",
    ["pack", "--dry-run", "--json", "--cache", path.join(repoRoot, ".npm-cache")],
    {
      cwd: repoRoot,
      encoding: "utf8",
    },
  );

  assert.equal(result.status, 0, result.stderr || result.stdout);

  const [packResult] = JSON.parse(result.stdout);
  const packedPaths = packResult.files.map((entry) => entry.path);

  assert.equal(packedPaths.some((entry) => entry.startsWith("src/")), false);
  assert.equal(packedPaths.includes("bin/kratos.js"), true);
});

test("packed root package installs the platform addon through optionalDependencies", async () => {
  const tempRoot = await fsp.mkdtemp(path.join(os.tmpdir(), "kratos-package-smoke-"));
  const outputPath = path.join(tempRoot, "run-cli-output.json");
  const rootPackage = await packRootTarball(tempRoot);
  const addonTarballs = await packFakeAddonTarballs(tempRoot, outputPath);
  const installRoot = path.join(tempRoot, "install-root");
  const addonPackageName = resolveAddonPackageName();

  await fsp.mkdir(installRoot, { recursive: true });
  await fsp.writeFile(
    path.join(installRoot, "package.json"),
    JSON.stringify(
      {
        name: "kratos-package-smoke",
        private: true,
        dependencies: {
          kratos: `file:${rootPackage.tarballPath}`,
        },
        overrides: Object.fromEntries(
          Object.entries(addonTarballs).map(([packageName, entry]) => [
            packageName,
            `file:${entry.tarballPath}`,
          ]),
        ),
      },
      null,
      2,
    ) + "\n",
  );

  const installResult = runCommand(
    "npm",
    [
      "install",
      "--cache",
      path.join(repoRoot, ".npm-cache"),
      "--no-package-lock",
      "--no-audit",
      "--ignore-scripts",
      "--fund=false",
    ],
    {
      cwd: installRoot,
    },
  );
  assert.equal(installResult.status, 0, installResult.stderr || installResult.stdout);

  const installedRootPackage = JSON.parse(
    await fsp.readFile(path.join(installRoot, "node_modules", "kratos", "package.json"), "utf8"),
  );
  const expectedAddonPackages = Object.keys(addonTarballs).sort();

  assert.equal(installedRootPackage.bin.kratos, "./bin/kratos.js");
  assert.deepEqual(installedRootPackage.optionalDependencies, packageJson.optionalDependencies);
  assert.deepEqual(Object.keys(packageJson.optionalDependencies).sort(), expectedAddonPackages);

  for (const packageName of expectedAddonPackages) {
    assert.equal(packageJson.optionalDependencies[packageName], packageJson.version);
  }

  const installedAddonPath = path.join(
    installRoot,
    "node_modules",
    ...addonPackageName.split("/"),
  );
  assert.equal(fs.existsSync(installedAddonPath), true);
  await assertWindowsCmdShimTargetsInstalledLauncher(installRoot);

  const binaryPath =
    process.platform === "win32"
      ? path.join(installRoot, "node_modules", ".bin", "kratos.cmd")
      : path.join(installRoot, "node_modules", ".bin", "kratos");

  const runResult = runCommand(binaryPath, ["scan", "fixtures/demo-app", "--json"], {
    cwd: installRoot,
    env: {
      ...process.env,
      KRATOS_PACKAGE_SMOKE_OUTPUT: outputPath,
    },
  });

  assert.equal(runResult.status, 0, runResult.stderr || runResult.stdout);

  const invocation = JSON.parse(await fsp.readFile(outputPath, "utf8"));
  assert.deepEqual(invocation.args, ["scan", "fixtures/demo-app", "--json"]);
});

async function assertWindowsCmdShimTargetsInstalledLauncher(installRoot) {
  const launcherPath = path.join(installRoot, "node_modules", "kratos", "bin", "kratos.js");
  const shimTarget = path.join(installRoot, "windows-bin", "kratos");
  const cmdShim = await loadCmdShim();

  await fsp.mkdir(path.dirname(shimTarget), { recursive: true });
  await cmdShim(launcherPath, shimTarget);

  const cmdShimBody = await fsp.readFile(`${shimTarget}.cmd`, "utf8");
  assert.match(cmdShimBody, /node_modules\\kratos\\bin\\kratos\.js/);
  assert.match(cmdShimBody, /%\*/);
}

async function packRootTarball(tempRoot) {
  const result = runCommand(
    "npm",
    ["pack", "--json", "--pack-destination", tempRoot, "--cache", path.join(repoRoot, ".npm-cache")],
    { cwd: repoRoot },
  );
  const [packResult] = JSON.parse(result.stdout);

  return {
    tarballPath: path.join(tempRoot, packResult.filename),
  };
}

async function packFakeAddonTarballs(tempRoot, outputPath) {
  const tarballs = {};

  for (const target of ADDON_TARGETS) {
    const packageName = resolveAddonPackageName(target.platform, target.arch);
    const packageSlug = packageName.replace("@kratos/", "");
    const packageRoot = path.join(tempRoot, `${packageSlug}-package`);

    await fsp.mkdir(packageRoot, { recursive: true });
    await fsp.writeFile(
      path.join(packageRoot, "package.json"),
      JSON.stringify(
        {
          name: packageName,
          version: packageJson.version,
          main: "./index.js",
          files: ["index.js"],
          os: [target.os],
          cpu: [target.cpu],
        },
        null,
        2,
      ) + "\n",
    );
    await fsp.writeFile(
      path.join(packageRoot, "index.js"),
      [
        "const fs = require('node:fs');",
        "",
        "exports.runCli = function runCli(args) {",
        "  const outputPath = process.env.KRATOS_PACKAGE_SMOKE_OUTPUT;",
        "  if (outputPath) {",
        "    fs.writeFileSync(outputPath, JSON.stringify({ args }) + '\\n');",
        "  }",
        "  return 0;",
        "};",
        "",
      ].join("\n"),
    );

    const result = runCommand(
      "npm",
      [
        "pack",
        packageRoot,
        "--json",
        "--pack-destination",
        tempRoot,
        "--cache",
        path.join(repoRoot, ".npm-cache"),
      ],
      { cwd: repoRoot },
    );
    const [packResult] = JSON.parse(result.stdout);

    const tarballPath = path.join(tempRoot, packResult.filename);
    const tarballBuffer = await fsp.readFile(tarballPath);

    tarballs[packageName] = {
      packageName,
      version: packageJson.version,
      tarballPath,
      tarballFileName: packResult.filename,
      shasum: crypto.createHash("sha1").update(tarballBuffer).digest("hex"),
      integrity: `sha512-${crypto.createHash("sha512").update(tarballBuffer).digest("base64")}`,
      os: [target.os],
      cpu: [target.cpu],
    };
  }

  assert.equal(fs.existsSync(outputPath), false);

  return tarballs;
}

async function loadCmdShim() {
  if (!cmdShimPromise) {
    const npmRootResult = runCommand("npm", ["root", "-g"], { cwd: repoRoot });
    const npmRoot = npmRootResult.stdout.trim();
    const cmdShimPath = [
      path.join(npmRoot, "npm", "node_modules", "cmd-shim", "lib", "index.js"),
      path.join(npmRoot, "cmd-shim", "lib", "index.js"),
    ].find((candidate) => fs.existsSync(candidate));

    assert.ok(cmdShimPath, `Could not locate npm cmd-shim from global npm root ${npmRoot}`);
    cmdShimPromise = import(pathToFileURL(cmdShimPath).href).then((module) => module.default ?? module);
  }

  return cmdShimPromise;
}

function runCommand(command, args, options) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    ...options,
  });

  if (result.error) {
    throw result.error;
  }

  return result;
}
