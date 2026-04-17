import path from "node:path";

import { SOURCE_EXTENSIONS } from "./constants.js";
import { fileExists, statOrNull } from "./fs.js";

export async function resolveImportTarget(request, importerPath, config) {
  if (request.startsWith("node:")) {
    return { kind: "external" };
  }

  if (request.startsWith(".")) {
    return resolveInternalPath(path.resolve(path.dirname(importerPath), request));
  }

  if (request.startsWith("/")) {
    return resolveInternalPath(path.resolve(config.root, `.${request}`));
  }

  for (const alias of config.pathAliases) {
    if (!matchesAlias(alias.alias, request)) {
      continue;
    }

    const resolved = await resolveAliasedImport(request, alias);
    return resolved ?? { kind: "missing-internal" };
  }

  if (config.baseUrl) {
    const candidate = await resolveInternalPath(path.resolve(config.baseUrl, request));
    if (candidate) {
      return candidate;
    }
  }

  return { kind: "external" };
}

async function resolveAliasedImport(request, alias) {
  const wildcard = alias.alias.includes("*");

  if (!wildcard) {
    return resolveInternalPath(alias.target);
  }

  const prefix = alias.alias.split("*")[0];
  const suffix = request.slice(prefix.length);
  return resolveInternalPath(path.resolve(alias.target, suffix));
}

function matchesAlias(alias, request) {
  if (alias.includes("*")) {
    return request.startsWith(alias.split("*")[0]);
  }

  return request === alias;
}

async function resolveInternalPath(basePath) {
  const direct = await resolveFile(basePath);
  if (direct) {
    return direct;
  }

  return { kind: "missing-internal" };
}

async function resolveFile(basePath) {
  const directStat = await statOrNull(basePath);

  if (directStat?.isFile()) {
    return {
      kind: isSourcePath(basePath) ? "source" : "asset",
      path: basePath,
    };
  }

  if (!path.extname(basePath)) {
    for (const extension of SOURCE_EXTENSIONS) {
      const candidate = `${basePath}${extension}`;

      if (await fileExists(candidate)) {
        return {
          kind: "source",
          path: candidate,
        };
      }
    }
  }

  const directoryStat = await statOrNull(basePath);

  if (directoryStat?.isDirectory()) {
    for (const extension of SOURCE_EXTENSIONS) {
      const indexPath = path.join(basePath, `index${extension}`);

      if (await fileExists(indexPath)) {
        return {
          kind: "source",
          path: indexPath,
        };
      }
    }
  }

  return null;
}

function isSourcePath(filePath) {
  return SOURCE_EXTENSIONS.includes(path.extname(filePath));
}
