import path from "node:path";

import { DEFAULT_CONFIG_FILENAME, DEFAULT_IGNORED_DIRS } from "./constants.js";
import { fileExists, readTextFile } from "./fs.js";
import { parseLooseJson } from "./jsonc.js";

export async function loadProjectConfig(root) {
  const packageJson = (await readLooseJsonFile(path.join(root, "package.json"))) ?? {};
  const tsconfig =
    (await readLooseJsonFile(path.join(root, "tsconfig.json"))) ??
    (await readLooseJsonFile(path.join(root, "jsconfig.json"))) ??
    {};
  const userConfig =
    (await readLooseJsonFile(path.join(root, DEFAULT_CONFIG_FILENAME))) ?? {};

  const compilerOptions = tsconfig.compilerOptions ?? {};
  const baseUrl =
    typeof compilerOptions.baseUrl === "string"
      ? path.resolve(root, compilerOptions.baseUrl)
      : null;

  return {
    root,
    baseUrl,
    roots: normalizeRoots(root, userConfig.roots),
    ignoredDirectories: new Set([
      ...DEFAULT_IGNORED_DIRS,
      ...(Array.isArray(userConfig.ignore) ? userConfig.ignore : []),
    ]),
    explicitEntries: new Set(
      (Array.isArray(userConfig.entry) ? userConfig.entry : []).map((entry) =>
        path.resolve(root, entry),
      ),
    ),
    packageEntries: new Set(collectPackageEntryFiles(root, packageJson)),
    pathAliases: normalizePathAliases(root, compilerOptions.paths, baseUrl),
  };
}

async function readLooseJsonFile(filePath) {
  if (!(await fileExists(filePath))) {
    return null;
  }

  return parseLooseJson(await readTextFile(filePath));
}

function normalizeRoots(root, roots) {
  if (!Array.isArray(roots) || roots.length === 0) {
    return [root];
  }

  return roots.map((value) => path.resolve(root, value));
}

function normalizePathAliases(root, rawPaths, baseUrl) {
  if (!rawPaths || typeof rawPaths !== "object") {
    return [];
  }

  return Object.entries(rawPaths)
    .flatMap(([alias, targets]) => {
      if (!Array.isArray(targets)) {
        return [];
      }

      const resolutionBase = baseUrl ?? root;

      return targets.map((target) => ({
        alias,
        target: path.resolve(resolutionBase, target.replace(/\*$/, "")),
      }));
    })
    .sort((left, right) => right.alias.length - left.alias.length);
}

function collectPackageEntryFiles(root, packageJson) {
  const entries = new Set();

  addEntryValue(entries, root, packageJson.main);
  addEntryValue(entries, root, packageJson.module);
  addEntryValue(entries, root, packageJson.types);

  if (typeof packageJson.bin === "string") {
    addEntryValue(entries, root, packageJson.bin);
  } else if (packageJson.bin && typeof packageJson.bin === "object") {
    for (const value of Object.values(packageJson.bin)) {
      addEntryValue(entries, root, value);
    }
  }

  collectExports(entries, root, packageJson.exports);

  return [...entries];
}

function collectExports(entries, root, value) {
  if (!value) {
    return;
  }

  if (typeof value === "string") {
    addEntryValue(entries, root, value);
    return;
  }

  if (Array.isArray(value)) {
    for (const item of value) {
      collectExports(entries, root, item);
    }
    return;
  }

  if (typeof value === "object") {
    for (const nestedValue of Object.values(value)) {
      collectExports(entries, root, nestedValue);
    }
  }
}

function addEntryValue(entries, root, value) {
  if (typeof value !== "string") {
    return;
  }

  entries.add(path.resolve(root, value));
}
