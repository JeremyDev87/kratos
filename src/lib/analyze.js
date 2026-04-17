import path from "node:path";

import { loadProjectConfig } from "./config.js";
import { collectSourceFiles } from "./discover.js";
import { readTextFile } from "./fs.js";
import { parseModuleSource } from "./parser.js";
import { resolveImportTarget } from "./resolve.js";

export async function analyzeProject(root) {
  const config = await loadProjectConfig(root);
  const files = await collectSourceFiles(config);
  const moduleEntries = await Promise.all(
    files.map(async (filePath) => {
      const source = await readTextFile(filePath);
      const parsed = parseModuleSource(source);
      const entrypointKind = detectEntrypointKind(filePath, config);

      return [
        filePath,
        {
          filePath,
          relativePath: toProjectPath(filePath, root),
          entrypointKind,
          imports: parsed.imports,
          exports: dedupeByName(parsed.exports),
          unusedImports: parsed.unusedImports,
          importedBy: new Set(),
          importers: [],
        },
      ];
    }),
  );

  const modules = new Map(moduleEntries);
  const brokenImports = [];
  const routeEntrypoints = [];

  for (const module of modules.values()) {
    if (module.entrypointKind?.startsWith("next")) {
      routeEntrypoints.push({
        file: module.filePath,
        kind: module.entrypointKind,
      });
    }

    module.resolvedImports = [];

    for (const entry of module.imports) {
      const resolution = await resolveImportTarget(entry.source, module.filePath, config);

      if (resolution.kind === "source" && modules.has(resolution.path)) {
        module.resolvedImports.push({
          kind: entry.kind,
          source: entry.source,
          target: resolution.path,
          specifiers: entry.specifiers,
        });

        const targetModule = modules.get(resolution.path);
        targetModule.importedBy.add(module.filePath);
        targetModule.importers.push({
          file: module.filePath,
          kind: entry.kind,
          specifiers: entry.specifiers,
        });
        continue;
      }

      if (resolution.kind === "missing-internal") {
        brokenImports.push({
          file: module.filePath,
          source: entry.source,
          kind: entry.kind,
        });
      }
    }
  }

  const orphanFiles = [];
  const deadExports = [];
  const unusedImports = [];
  const deletionCandidates = [];

  for (const module of modules.values()) {
    if (module.unusedImports.length > 0) {
      for (const entry of module.unusedImports) {
        unusedImports.push({
          file: module.filePath,
          source: entry.source,
          local: entry.local,
          imported: entry.imported,
        });
      }
    }

    if (module.importedBy.size === 0 && !module.entrypointKind) {
      const classification = classifyOrphan(module.relativePath);
      orphanFiles.push({
        file: module.filePath,
        kind: classification.kind,
        reason: classification.reason,
      });
      deletionCandidates.push({
        file: module.filePath,
        reason: classification.reason,
        confidence: classification.confidence,
        safe: true,
      });
    }

    const exportUsage = summarizeExportUsage(module);
    const shouldSkipDeadExports =
      module.entrypointKind || exportUsage.usesNamespace || exportUsage.usesUnknown;

    if (!shouldSkipDeadExports) {
      for (const exported of module.exports) {
        if (exported.name === "*" || exportUsage.usedNames.has(exported.name)) {
          continue;
        }

        deadExports.push({
          file: module.filePath,
          exportName: exported.name,
        });
      }
    }
  }

  const summary = {
    filesScanned: modules.size,
    entrypoints: [...modules.values()].filter((module) => Boolean(module.entrypointKind))
      .length,
    brokenImports: brokenImports.length,
    orphanFiles: orphanFiles.length,
    deadExports: deadExports.length,
    unusedImports: unusedImports.length,
    routeEntrypoints: routeEntrypoints.length,
    deletionCandidates: deletionCandidates.length,
  };

  return {
    version: 1,
    generatedAt: new Date().toISOString(),
    root,
    summary,
    findings: {
      brokenImports,
      orphanFiles,
      deadExports,
      unusedImports,
      routeEntrypoints,
      deletionCandidates,
    },
    modules: [...modules.values()].map((module) => ({
      file: module.filePath,
      relativePath: module.relativePath,
      entrypointKind: module.entrypointKind,
      importedByCount: module.importedBy.size,
      importCount: module.resolvedImports.length,
      exportCount: module.exports.length,
    })),
  };
}

function summarizeExportUsage(module) {
  const usedNames = new Set();
  let usesNamespace = false;
  let usesUnknown = false;

  for (const importer of module.importers) {
    for (const specifier of importer.specifiers) {
      if (specifier.kind === "namespace") {
        usesNamespace = true;
        continue;
      }

      if (specifier.kind === "unknown") {
        usesUnknown = true;
        continue;
      }

      if (specifier.imported) {
        usedNames.add(specifier.imported);
      }
    }
  }

  return { usedNames, usesNamespace, usesUnknown };
}

function classifyOrphan(relativePath) {
  const normalized = relativePath.toLowerCase();
  const fileName = relativePath.split("/").pop() ?? relativePath;

  if (
    normalized.includes("/components/") ||
    /^[A-Z]/.test(fileName)
  ) {
    return {
      kind: "orphan-component",
      reason: "Component-like module has no inbound references.",
      confidence: 0.92,
    };
  }

  if (normalized.includes("/routes/") || /page|route/.test(fileName.toLowerCase())) {
    return {
      kind: "orphan-route-module",
      reason: "Route-like module is not connected to any router entry.",
      confidence: 0.84,
    };
  }

  return {
    kind: "orphan-module",
    reason: "Module has no inbound references and is not treated as an entrypoint.",
    confidence: 0.88,
  };
}

function detectEntrypointKind(filePath, config) {
  if (config.explicitEntries.has(filePath)) {
    return "user-entry";
  }

  if (config.packageEntries.has(filePath)) {
    return "package-entry";
  }

  const relativePath = toProjectPath(filePath, config.root);
  const normalized = relativePath.replace(/\\/g, "/");

  if (/^(app\/).+\/(page|route|layout|loading|error|not-found)\.[^.]+$/.test(normalized)) {
    return "next-app-route";
  }

  if (/^(pages\/).+\.[^.]+$/.test(normalized)) {
    return "next-pages-route";
  }

  if (/^(src\/)?(main|index|bootstrap|cli)\.[^.]+$/.test(normalized)) {
    return "app-entry";
  }

  if (
    /(^|\/)(next|vite|webpack|rollup|vitest|jest|postcss|tailwind|babel|playwright|cypress)\.config\.[^.]+$/.test(
      normalized,
    )
  ) {
    return "tooling-entry";
  }

  if (/^(middleware|instrumentation)\.[^.]+$/.test(normalized)) {
    return "framework-entry";
  }

  return null;
}

function dedupeByName(exports) {
  const seen = new Set();
  const deduped = [];

  for (const entry of exports) {
    const key = `${entry.kind}:${entry.name}`;
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    deduped.push(entry);
  }

  return deduped;
}

function toProjectPath(filePath, root) {
  return path.relative(root, filePath).split(path.sep).join("/");
}
