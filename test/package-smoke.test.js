import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import fsp from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath, pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";

import { resolveAddonPackageName } from "../bin/kratos.js";

const require = createRequire(import.meta.url);
const packageJson = require("../package.json");
const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
let cmdShimPromise;
const npmCommand = "npm";
const npmCommandOptions = process.platform === "win32" ? { shell: true } : {};
const ADDON_TARGETS = [
  { platform: "darwin", arch: "arm64", os: "darwin", cpu: "arm64" },
  { platform: "darwin", arch: "x64", os: "darwin", cpu: "x64" },
  { platform: "linux", arch: "x64", os: "linux", cpu: "x64" },
  { platform: "linux", arch: "arm64", os: "linux", cpu: "arm64" },
  { platform: "win32", arch: "x64", os: "win32", cpu: "x64" },
];

test("root npm pack dry-run excludes src runtime files", () => {
  const result = runNpmCommand(
    ["pack", "--dry-run", "--json", "--cache", path.join(repoRoot, ".npm-cache")],
    {
      cwd: repoRoot,
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
  const addonTarballs = await packAddonTarballs(tempRoot, { outputPath });
  const installRoot = await installPackedRootPackage(tempRoot, rootPackage, addonTarballs);
  const addonPackageName = resolveAddonPackageName();

  const installedRootPackage = JSON.parse(
    await fsp.readFile(
      path.join(installRoot, "node_modules", ...packageJson.name.split("/"), "package.json"),
      "utf8",
    ),
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

  const runResult = runInstalledKratos(installRoot, ["scan", "fixtures/demo-app", "--json"], {
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

test("packed root package boots the actual native addon for the current platform", async (t) => {
  const nativeLibraryPath = process.env.KRATOS_PACKAGE_SMOKE_NATIVE_LIB;

  if (!nativeLibraryPath) {
    t.skip("KRATOS_PACKAGE_SMOKE_NATIVE_LIB is not set");
    return;
  }

  const resolvedNativeLibraryPath = path.resolve(repoRoot, nativeLibraryPath);
  assert.equal(
    fs.existsSync(resolvedNativeLibraryPath),
    true,
    `Expected native library at ${resolvedNativeLibraryPath}`,
  );

  const tempRoot = await fsp.mkdtemp(path.join(os.tmpdir(), "kratos-package-native-smoke-"));
  const rootPackage = await packRootTarball(tempRoot);
  const addonTarballs = await packAddonTarballs(tempRoot, {
    nativeLibraryPath: resolvedNativeLibraryPath,
  });
  const installRoot = await installPackedRootPackage(tempRoot, rootPackage, addonTarballs);
  const addonPackageName = resolveAddonPackageName();
  const installedAddonPath = path.join(installRoot, "node_modules", ...addonPackageName.split("/"));
  const demoAppPath = path.join(repoRoot, "fixtures", "demo-app");

  assert.equal(fs.existsSync(path.join(installedAddonPath, "kratos.node")), true);
  await assertWindowsCmdShimTargetsInstalledLauncher(installRoot);

  const runResult = runInstalledKratos(installRoot, ["scan", demoAppPath, "--json"], {
    cwd: installRoot,
  });

  assert.equal(runResult.status, 0, runResult.stderr || runResult.stdout);

  const report = JSON.parse(runResult.stdout);
  assert.equal(report.schemaVersion, 2);
  assert.equal(path.resolve(report.project.root), demoAppPath);
});

test("packed root package can fail CI gates on findings", async (t) => {
  const nativeLibraryPath = process.env.KRATOS_PACKAGE_SMOKE_NATIVE_LIB;

  if (!nativeLibraryPath) {
    t.skip("KRATOS_PACKAGE_SMOKE_NATIVE_LIB is not set");
    return;
  }

  const resolvedNativeLibraryPath = path.resolve(repoRoot, nativeLibraryPath);
  const tempRoot = await fsp.mkdtemp(path.join(os.tmpdir(), "kratos-package-gate-smoke-"));
  const rootPackage = await packRootTarball(tempRoot);
  const addonTarballs = await packAddonTarballs(tempRoot, {
    nativeLibraryPath: resolvedNativeLibraryPath,
  });
  const installRoot = await installPackedRootPackage(tempRoot, rootPackage, addonTarballs);
  const demoAppPath = path.join(repoRoot, "fixtures", "demo-app");

  const runResult = runInstalledKratos(
    installRoot,
    ["scan", demoAppPath, "--fail-on", "broken-imports,deletion-candidates"],
    {
      cwd: installRoot,
    },
  );

  assert.equal(runResult.status, 2, runResult.stderr || runResult.stdout);
  assert.match(runResult.stdout, /Kratos scan complete\./);
  assert.match(runResult.stdout, /Gate status: failed/);
  assert.match(runResult.stdout, /broken imports: 1/);
});

async function assertWindowsCmdShimTargetsInstalledLauncher(installRoot) {
  const launcherPath = path.join(
    installRoot,
    "node_modules",
    ...packageJson.name.split("/"),
    "bin",
    "kratos.js",
  );
  let cmdShimBody;

  if (process.platform === "win32") {
    const installedShimPath = path.join(installRoot, "node_modules", ".bin", "kratos.cmd");
    assert.equal(
      fs.existsSync(installedShimPath),
      true,
      `Expected installed Windows cmd shim at ${installedShimPath}`,
    );
    cmdShimBody = await fsp.readFile(installedShimPath, "utf8");
  } else {
    const shimTarget = path.join(installRoot, "windows-bin", "kratos");
    const cmdShim = await loadCmdShim();

    await fsp.mkdir(path.dirname(shimTarget), { recursive: true });
    await cmdShim(launcherPath, shimTarget);
    cmdShimBody = await fsp.readFile(`${shimTarget}.cmd`, "utf8");
  }

  const packagePathPattern = packageJson.name.split("/").map(escapeForRegex).join("\\\\");
  assert.match(
    cmdShimBody,
    new RegExp(`(?:node_modules|\\.{2})\\\\${packagePathPattern}\\\\bin\\\\kratos\\.js`, "i"),
  );
  assert.match(cmdShimBody, /%\*/);
}

async function packRootTarball(tempRoot) {
  const result = runNpmCommand(
    ["pack", "--json", "--pack-destination", tempRoot, "--cache", path.join(repoRoot, ".npm-cache")],
    { cwd: repoRoot },
  );
  const [packResult] = JSON.parse(result.stdout);

  return {
    tarballPath: path.join(tempRoot, packResult.filename),
  };
}

async function installPackedRootPackage(tempRoot, rootPackage, addonTarballs) {
  const installRoot = path.join(tempRoot, "install-root");

  await fsp.mkdir(installRoot, { recursive: true });
  await fsp.writeFile(
    path.join(installRoot, "package.json"),
    JSON.stringify(
      {
        name: "kratos-package-smoke",
        private: true,
        dependencies: {
          [packageJson.name]: `file:${rootPackage.tarballPath}`,
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

  const installResult = runNpmCommand(
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

  return installRoot;
}

async function packAddonTarballs(tempRoot, { outputPath, nativeLibraryPath } = {}) {
  const tarballs = {};
  const currentAddonPackageName = nativeLibraryPath ? resolveAddonPackageName() : null;

  for (const target of ADDON_TARGETS) {
    const packageName = resolveAddonPackageName(target.platform, target.arch);
    const packageSlug = packageName.split("/").pop();
    const packageRoot = path.join(tempRoot, `${packageSlug}-package`);
    const useNativeAddon = packageName === currentAddonPackageName;

    await fsp.mkdir(packageRoot, { recursive: true });
    if (useNativeAddon) {
      await fsp.copyFile(nativeLibraryPath, path.join(packageRoot, "kratos.node"));
    }
    await fsp.writeFile(
      path.join(packageRoot, "package.json"),
      JSON.stringify(
        {
          name: packageName,
          version: packageJson.version,
          main: useNativeAddon ? "./kratos.node" : "./index.js",
          files: useNativeAddon ? ["kratos.node"] : ["index.js"],
          os: [target.os],
          cpu: [target.cpu],
        },
        null,
        2,
      ) + "\n",
    );
    if (!useNativeAddon) {
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
    }

    const result = runNpmCommand(
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

    tarballs[packageName] = {
      tarballPath,
    };
  }

  if (outputPath) {
    assert.equal(fs.existsSync(outputPath), false);
  }

  return tarballs;
}

async function loadCmdShim() {
  if (!cmdShimPromise) {
    const npmRootResult = runNpmCommand(["root", "-g"], { cwd: repoRoot });
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

function runNpmCommand(args, options) {
  return runCommand(npmCommand, args, {
    ...options,
    ...npmCommandOptions,
  });
}

function runInstalledKratos(installRoot, args, options = {}) {
  const binaryPath =
    process.platform === "win32"
      ? path.join(installRoot, "node_modules", ".bin", "kratos.cmd")
      : path.join(installRoot, "node_modules", ".bin", "kratos");

  if (process.platform === "win32") {
    return runCommand(binaryPath, args, {
      shell: true,
      ...options,
    });
  }

  return runCommand(binaryPath, args, options);
}

function escapeForRegex(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
