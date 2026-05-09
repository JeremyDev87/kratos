#!/usr/bin/env node

import fs from "node:fs/promises";
import crypto from "node:crypto";
import path from "node:path";
import { pathToFileURL } from "node:url";

import { assertReleaseUpgrade, resolveReleasePlan } from "./lib/release.mjs";

export function normalizeReleaseTag(input) {
  return input.startsWith("v") ? input : `v${input}`;
}

export function updatePackageManifest(pkg, version) {
  const nextPkg = {
    ...pkg,
    version,
  };

  if (pkg.optionalDependencies && typeof pkg.optionalDependencies === "object") {
    nextPkg.optionalDependencies = Object.fromEntries(
      Object.entries(pkg.optionalDependencies).map(([name]) => [name, version]),
    );
  }

  return nextPkg;
}

export function readCargoWorkspacePackageVersion(cargoToml) {
  let inWorkspacePackage = false;
  let sawWorkspacePackage = false;

  for (const line of cargoToml.split("\n")) {
    if (/^\s*\[workspace\.package\]\s*(?:#.*)?$/.test(line)) {
      inWorkspacePackage = true;
      sawWorkspacePackage = true;
      continue;
    }

    if (inWorkspacePackage && /^\s*\[[^\]]+\]\s*(?:#.*)?$/.test(line)) {
      inWorkspacePackage = false;
    }

    if (!inWorkspacePackage) {
      continue;
    }

    const versionMatch = /^(\s*)version\s*=\s*"([^"]+)"(?:\s*#.*)?$/.exec(line);
    if (versionMatch) {
      return versionMatch[2];
    }
  }

  if (!sawWorkspacePackage) {
    throw new Error("Cargo.toml is missing a [workspace.package] section");
  }

  throw new Error("Cargo.toml is missing [workspace.package] version");
}

export function updateCargoWorkspacePackageVersion(cargoToml, version) {
  const currentVersion = readCargoWorkspacePackageVersion(cargoToml);
  let inWorkspacePackage = false;
  let updatedVersion = false;

  const contents = cargoToml
    .split("\n")
    .map((line) => {
      if (/^\s*\[workspace\.package\]\s*(?:#.*)?$/.test(line)) {
        inWorkspacePackage = true;
        return line;
      }

      if (inWorkspacePackage && /^\s*\[[^\]]+\]\s*(?:#.*)?$/.test(line)) {
        inWorkspacePackage = false;
      }

      if (!inWorkspacePackage || updatedVersion) {
        return line;
      }

      const versionMatch = /^(\s*version\s*=\s*)"([^"]+)"(\s*(?:#.*)?)$/.exec(line);
      if (!versionMatch) {
        return line;
      }

      updatedVersion = true;
      return `${versionMatch[1]}"${version}"${versionMatch[3]}`;
    })
    .join("\n");

  return {
    contents,
    currentVersion,
    changed: currentVersion !== version,
  };
}

export function readCargoLockWorkspacePackageVersions(cargoLock) {
  return cargoLock
    .split(/(?=^\[\[package\]\]\r?\n)/m)
    .flatMap((block) => {
      if (!block.startsWith("[[package]]") || /^\s*source\s*=/m.test(block)) {
        return [];
      }

      const nameMatch = /^name\s*=\s*"([^"]+)"$/m.exec(block);
      const versionMatch = /^version\s*=\s*"([^"]+)"$/m.exec(block);

      if (!nameMatch || !versionMatch) {
        return [];
      }

      return [
        {
          name: nameMatch[1],
          version: versionMatch[1],
        },
      ];
    });
}

export function updateCargoLockWorkspacePackageVersions(cargoLock, version) {
  const changedPackageNames = [];
  const contents = cargoLock
    .split(/(?=^\[\[package\]\]\r?\n)/m)
    .map((block) => {
      if (!block.startsWith("[[package]]") || /^\s*source\s*=/m.test(block)) {
        return block;
      }

      const nameMatch = /^name\s*=\s*"([^"]+)"$/m.exec(block);
      const versionMatch = /^version\s*=\s*"([^"]+)"$/m.exec(block);

      if (!nameMatch || !versionMatch || versionMatch[1] === version) {
        return block;
      }

      changedPackageNames.push(nameMatch[1]);
      return block.replace(/^version\s*=\s*"[^"]+"$/m, `version = "${version}"`);
    })
    .join("");

  return {
    contents,
    changedPackageNames,
    changed: changedPackageNames.length > 0,
  };
}

export function createManualBumpBranchName(input, baseRef = "master") {
  const normalizedTag = normalizeReleaseTag(input);
  const branchBase = baseRef
    .replace(/[./+]/g, "-")
    .replace(/[^0-9A-Za-z-]/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
  const branchVersion = normalizedTag
    .slice(1)
    .replace(/[.+]/g, "-")
    .replace(/[^0-9A-Za-z-]/g, "-");
  const branchHash = crypto
    .createHash("sha256")
    .update(`${baseRef}\0${normalizedTag}`)
    .digest("hex")
    .slice(0, 8);

  return `codex/manual-bump-${branchBase}-v${branchVersion}-${branchHash}`;
}

async function readOptionalFile(filePath) {
  try {
    return await fs.readFile(filePath, "utf8");
  } catch (error) {
    if (error && error.code === "ENOENT") {
      return null;
    }

    throw error;
  }
}

export async function bumpPackageVersion(input, packageJsonPath = "package.json", options = {}) {
  const normalizedTag = normalizeReleaseTag(input);
  const plan = resolveReleasePlan(normalizedTag);
  const manifestPath = path.resolve(packageJsonPath);
  const workspaceRoot = path.dirname(manifestPath);
  const cargoTomlPath = path.resolve(options.cargoTomlPath ?? path.join(workspaceRoot, "Cargo.toml"));
  const cargoLockPath = path.resolve(options.cargoLockPath ?? path.join(workspaceRoot, "Cargo.lock"));
  const [manifestText, cargoTomlText, cargoLockText] = await Promise.all([
    fs.readFile(manifestPath, "utf8"),
    fs.readFile(cargoTomlPath, "utf8"),
    readOptionalFile(cargoLockPath),
  ]);
  const manifest = JSON.parse(manifestText);
  const cargoTomlUpdate = updateCargoWorkspacePackageVersion(cargoTomlText, plan.version);
  const cargoLockUpdate =
    cargoLockText === null
      ? {
          contents: null,
          changedPackageNames: [],
          changed: false,
        }
      : updateCargoLockWorkspacePackageVersions(cargoLockText, plan.version);

  assertReleaseUpgrade(manifest.version, plan.version);
  assertReleaseUpgrade(cargoTomlUpdate.currentVersion, plan.version);

  const nextManifest = updatePackageManifest(manifest, plan.version);

  await fs.writeFile(manifestPath, `${JSON.stringify(nextManifest, null, 2)}\n`);
  await fs.writeFile(cargoTomlPath, cargoTomlUpdate.contents);

  if (cargoLockUpdate.changed) {
    await fs.writeFile(cargoLockPath, cargoLockUpdate.contents);
  }

  return {
    manifestPath,
    cargoTomlPath,
    cargoLockPath: cargoLockText === null ? null : cargoLockPath,
    cargoLockPackageNames: cargoLockUpdate.changedPackageNames,
    version: plan.version,
    tag: plan.tag,
    isPrerelease: plan.isPrerelease,
  };
}

async function main() {
  const input = process.argv[2];
  const packageJsonPath = process.argv[3] ?? "package.json";

  if (!input) {
    console.error("Usage: node ./scripts/bump-release-version.mjs <tag-or-version> [package-json-path]");
    process.exit(1);
  }

  const result = await bumpPackageVersion(input, packageJsonPath);

  console.log(`tag=${result.tag}`);
  console.log(`version=${result.version}`);
  console.log(`isPrerelease=${result.isPrerelease}`);
  console.log(`manifestPath=${result.manifestPath}`);
  console.log(`cargoTomlPath=${result.cargoTomlPath}`);
  if (result.cargoLockPath) {
    console.log(`cargoLockPath=${result.cargoLockPath}`);
    console.log(`cargoLockPackageNames=${result.cargoLockPackageNames.join(",")}`);
  }
}

const isDirectExecution =
  process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href;

if (isDirectExecution) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  });
}
