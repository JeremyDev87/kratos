#!/usr/bin/env node

import fs from "node:fs";
import process from "node:process";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const require = createRequire(import.meta.url);

export function resolveAddonPackageName(
  platform = process.platform,
  arch = process.arch,
) {
  if (platform === "darwin" && arch === "arm64") {
    return "@jeremyfellaz/kratos-darwin-arm64";
  }
  if (platform === "darwin" && arch === "x64") {
    return "@jeremyfellaz/kratos-darwin-x64";
  }
  if (platform === "linux" && arch === "x64") {
    return "@jeremyfellaz/kratos-linux-x64-gnu";
  }
  if (platform === "linux" && arch === "arm64") {
    return "@jeremyfellaz/kratos-linux-arm64-gnu";
  }
  if (platform === "win32" && arch === "x64") {
    return "@jeremyfellaz/kratos-win32-x64-msvc";
  }

  throw new Error(`Unsupported platform/arch for Kratos native addon: ${platform}/${arch}`);
}

export function loadNativeBinding({
  platform = process.platform,
  arch = process.arch,
  requireFn = require,
} = {}) {
  const packageName = resolveAddonPackageName(platform, arch);

  try {
    const binding = requireFn(packageName);
    return { binding, packageName };
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to load native addon package ${packageName}: ${detail}`);
  }
}

export function runLauncher(argv, options = {}) {
  const {
    platform = process.platform,
    arch = process.arch,
    requireFn = require,
    stderr = process.stderr,
  } = options;
  const args = argv.slice(2);

  try {
    const { binding, packageName } = loadNativeBinding({ platform, arch, requireFn });
    const runCli =
      typeof binding?.runCli === "function"
        ? binding.runCli
        : typeof binding?.default?.runCli === "function"
          ? binding.default.runCli
          : null;

    if (!runCli) {
      throw new Error(`Native addon package ${packageName} does not export runCli`);
    }

    const exitCode = runCli(args);
    return Number.isInteger(exitCode) ? exitCode : 1;
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    stderr.write(`Kratos failed: ${detail}\n`);
    return 1;
  }
}

export function isDirectExecution(metaUrl = import.meta.url, argv = process.argv) {
  const entryPath = argv[1];
  if (!entryPath) {
    return false;
  }

  return normalizeExecutionPath(entryPath) === normalizeExecutionPath(fileURLToPath(metaUrl));
}

function normalizeExecutionPath(rawPath) {
  const absolutePath = path.resolve(rawPath);

  try {
    return fs.realpathSync.native(absolutePath);
  } catch {
    return absolutePath;
  }
}

if (isDirectExecution()) {
  process.exitCode = runLauncher(process.argv);
}
