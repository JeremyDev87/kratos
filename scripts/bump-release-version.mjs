#!/usr/bin/env node

import fs from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

import { resolveReleasePlan } from "./lib/release.mjs";

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

export async function bumpPackageVersion(input, packageJsonPath = "package.json") {
  const normalizedTag = normalizeReleaseTag(input);
  const plan = resolveReleasePlan(normalizedTag);
  const manifestPath = path.resolve(packageJsonPath);
  const manifest = JSON.parse(await fs.readFile(manifestPath, "utf8"));
  const nextManifest = updatePackageManifest(manifest, plan.version);

  await fs.writeFile(manifestPath, `${JSON.stringify(nextManifest, null, 2)}\n`);

  return {
    manifestPath,
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
}

const isDirectExecution =
  process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href;

if (isDirectExecution) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  });
}
